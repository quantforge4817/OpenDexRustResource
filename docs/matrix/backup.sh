#!/usr/bin/env bash
#
# backup.sh — Daily RocksDB backup for Conduit Matrix homeserver
#
# Usage: ./backup.sh
#
# This script:
# 1. Creates a snapshot of the RocksDB database
# 2. Compresses the backup
# 3. Optionally uploads to remote S3-compatible storage
# 4. Cleans up old backups (keeps last 7 days locally)
#
# Set up as a cron job:
#   0 2 * * * /path/to/backup.sh
#

set -euo pipefail

# --- Configuration ---
CONDUIT_DATA_DIR="/var/lib/conduit"
BACKUP_DIR="/var/backups/conduit"
RETENTION_DAYS=7
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/conduit_backup_${TIMESTAMP}.tar.gz"

# Optional: S3-compatible remote storage
# Set these environment variables to enable remote backups:
#   BACKUP_S3_BUCKET="my-backup-bucket"
#   BACKUP_S3_KEY="your-access-key"
#   BACKUP_S3_SECRET="your-secret-key"
#   BACKUP_S3_ENDPOINT="https://s3.example.com" (optional, for non-AWS)
REMOTE_BACKUP="${BACKUP_REMOTE:-false}"

echo "============================================"
echo " Conduit Matrix Backup"
echo "============================================"
echo "Timestamp: ${TIMESTAMP}"
echo "Data dir:  ${CONDUIT_DATA_DIR}"
echo "Backup:    ${BACKUP_FILE}"
echo "--------------------------------------------"

# --- Check root ---
if [[ $EUID -ne 0 ]]; then
  echo "ERROR: This script should be run as root (sudo)."
  exit 1
fi

# --- Create backup directory ---
mkdir -p "${BACKUP_DIR}"

# --- Stop Conduit temporarily for consistent backup ---
echo ""
echo "Stopping Conduit for consistent backup..."
if systemctl is-active --quiet conduit; then
  systemctl stop conduit
  echo "Conduit stopped."
else
  echo "Conduit is not running. Proceeding with backup..."
fi

# --- Create backup ---
echo ""
echo "Creating RocksDB backup..."

if [[ -d "${CONDUIT_DATA_DIR}/db" ]]; then
  # Compress the entire data directory
  tar czf "${BACKUP_FILE}" -C "${CONDUIT_DATA_DIR}" .
  BACKUP_SIZE=$(du -h "${BACKUP_FILE}" | cut -f1)
  echo "Backup created: ${BACKUP_FILE} (${BACKUP_SIZE})"
else
  echo "WARNING: RocksDB directory not found at ${CONDUIT_DATA_DIR}/db"
  echo "Backing up entire Conduit data directory..."
  tar czf "${BACKUP_FILE}" -C "$(dirname ${CONDUIT_DATA_DIR})" "$(basename ${CONDUIT_DATA_DIR})"
  BACKUP_SIZE=$(du -h "${BACKUP_FILE}" | cut -f1)
  echo "Backup created: ${BACKUP_FILE} (${BACKUP_SIZE})"
fi

# --- Restart Conduit ---
echo ""
echo "Restarting Conduit..."
systemctl start conduit
if systemctl is-active --quiet conduit; then
  echo "Conduit restarted successfully."
else
  echo "WARNING: Conduit failed to restart. Check logs:"
  echo "  journalctl -u conduit -n 50"
fi

# --- Upload to remote storage (optional) ---
if [[ "${REMOTE_BACKUP}" == "true" ]]; then
  echo ""
  echo "Uploading to remote storage..."

  if [[ -n "${BACKUP_S3_BUCKET:-}" ]] && [[ -n "${BACKUP_S3_KEY:-}" ]] && [[ -n "${BACKUP_S3_SECRET:-}" ]]; then
    # Use AWS CLI if available
    if command -v aws &> /dev/null; then
      export AWS_ACCESS_KEY_ID="${BACKUP_S3_KEY}"
      export AWS_SECRET_ACCESS_KEY="${BACKUP_S3_SECRET}"

      if [[ -n "${BACKUP_S3_ENDPOINT:-}" ]]; then
        aws --endpoint-url "${BACKUP_S3_ENDPOINT}" s3 cp "${BACKUP_FILE}" "s3://${BACKUP_S3_BUCKET}/$(basename ${BACKUP_FILE})"
      else
        aws s3 cp "${BACKUP_FILE}" "s3://${BACKUP_S3_BUCKET}/$(basename ${BACKUP_FILE})"
      fi
      echo "Uploaded to S3: ${BACKUP_S3_BUCKET}/$(basename ${BACKUP_FILE})"
    elif command -v rclone &> /dev/null; then
      # Use rclone as fallback
      rclone copy "${BACKUP_FILE}" "remote:${BACKUP_S3_BUCKET}/"
      echo "Uploaded via rclone."
    else
      echo "WARNING: No S3 client found (aws or rclone). Install one to enable remote backups."
    fi
  else
    echo "WARNING: Remote backup enabled but S3 credentials not set."
    echo "Set BACKUP_S3_BUCKET, BACKUP_S3_KEY, and BACKUP_S3_SECRET environment variables."
  fi
else
  echo ""
  echo "Remote backup disabled. Set BACKUP_REMOTE=true to enable."
fi

# --- Clean up old backups ---
echo ""
echo "Cleaning up backups older than ${RETENTION_DAYS} days..."
find "${BACKUP_DIR}" -name "conduit_backup_*.tar.gz" -mtime +${RETENTION_DAYS} -delete
REMAINING=$(find "${BACKUP_DIR}" -name "conduit_backup_*.tar.gz" | wc -l)
echo "Remaining local backups: ${REMAINING}"

# --- Summary ---
echo ""
echo "============================================"
echo " Backup Complete"
echo "============================================"
echo "File: ${BACKUP_FILE}"
echo "Size: ${BACKUP_SIZE:-unknown}"
echo "Remote: ${REMOTE_BACKUP}"
echo "Retention: ${RETENTION_DAYS} days"
echo "============================================"
