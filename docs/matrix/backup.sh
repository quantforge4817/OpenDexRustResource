#!/bin/bash
#
# OpenDex Matrix Server Backup Script
# Daily backup of Conduit RocksDB database
#
# Usage: sudo bash backup.sh
# Recommended: Run daily via cron at 2 AM
#   0 2 * * * /path/to/backup.sh
#

set -euo pipefail

# Configuration
BACKUP_DIR="/var/backups/conduit"
DOCKER_CONTAINER="conduit"
DATA_VOLUME="docs-matrix_conduit-data"
RETENTION_DAYS=7  # Keep backups for 7 days
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="conduit_backup_${TIMESTAMP}.tar.gz"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN} Conduit Database Backup${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Timestamp: $TIMESTAMP"
echo "Backup directory: $BACKUP_DIR"
echo "Retention: $RETENTION_DAYS days"
echo ""

# Create backup directory if it doesn't exist
mkdir -p "$BACKUP_DIR"

# ========================================
# Step 1: Stop Conduit container
# ========================================
echo -e "${GREEN}[1/4] Stopping Conduit container...${NC}"

if docker ps --format '{{.Names}}' | grep -q "^${DOCKER_CONTAINER}$"; then
    docker stop "$DOCKER_CONTAINER"
    echo -e "${GREEN}✓ Container stopped${NC}"
else
    echo -e "${YELLOW}Warning: Container not running${NC}"
fi

# ========================================
# Step 2: Backup the RocksDB data
# ========================================
echo -e "\n${GREEN}[2/4] Creating backup...${NC}"

# Create a temporary directory for the backup
TEMP_DIR=$(mktemp -d)

# Extract the volume data
docker run --rm \
    -v "${DATA_VOLUME}:/data:ro" \
    -v "${TEMP_DIR}:/backup" \
    alpine \
    tar -czf "/backup/${BACKUP_NAME}" -C /data .

# Move the backup to the backup directory
mv "${TEMP_DIR}/${BACKUP_NAME}" "$BACKUP_DIR/"

# Clean up temp directory
rm -rf "$TEMP_DIR"

# Verify the backup exists and has size
if [ -f "$BACKUP_DIR/$BACKUP_NAME" ]; then
    BACKUP_SIZE=$(du -h "$BACKUP_DIR/$BACKUP_NAME" | cut -f1)
    echo -e "${GREEN}✓ Backup created: ${BACKUP_NAME} (${BACKUP_SIZE})${NC}"
else
    echo -e "${RED}✗ Backup failed: file not created${NC}"
    docker start "$DOCKER_CONTAINER" 2>/dev/null || true
    exit 1
fi

# ========================================
# Step 3: Restart Conduit container
# ========================================
echo -e "\n${GREEN}[3/4] Restarting Conduit container...${NC}"

docker start "$DOCKER_CONTAINER" 2>/dev/null || {
    echo -e "${YELLOW}Warning: Could not start container (was not running before)${NC}"
}

echo -e "${GREEN}✓ Container restarted${NC}"

# ========================================
# Step 4: Clean up old backups
# ========================================
echo -e "\n${GREEN}[4/4] Cleaning up old backups...${NC}"

OLD_BACKUPS=$(find "$BACKUP_DIR" -name "conduit_backup_*.tar.gz" -mtime +${RETENTION_DAYS} -type f)
if [ -n "$OLD_BACKUPS" ]; then
    echo "$OLD_BACKUPS" | xargs rm -f
    echo -e "${GREEN}✓ Old backups removed (older than $RETENTION_DAYS days)${NC}"
else
    echo "No old backups to remove."
fi

# ========================================
# Optional: Remote backup sync
# ========================================
# Uncomment and configure one of the following to sync backups to remote storage

# Option 1: rsync to a remote server
# echo -e "\n${GREEN}Syncing to remote storage...${NC}"
# rsync -avz -e ssh "$BACKUP_DIR/$BACKUP_NAME" user@remote-server:/path/to/backups/
# echo -e "${GREEN}✓ Remote sync complete${NC}"

# Option 2: AWS S3
# if command -v aws &> /dev/null; then
#     echo -e "\n${GREEN}Uploading to S3...${NC}"
#     aws s3 cp "$BACKUP_DIR/$BACKUP_NAME" s3://your-bucket-name/conduit-backups/
#     echo -e "${GREEN}✓ S3 upload complete${NC}"
# fi

# Option 3: Rclone
# if command -v rclone &> /dev/null; then
#     echo -e "\n${GREEN}Uploading with rclone...${NC}"
#     rclone copy "$BACKUP_DIR/$BACKUP_NAME" remote:bucket/conduit-backups/
#     echo -e "${GREEN}✓ Rclone upload complete${NC}"
# fi

# ========================================
# Backup Summary
# ========================================
echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN} Backup Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Backup file: $BACKUP_DIR/$BACKUP_NAME"
echo "Backup size: $BACKUP_SIZE"
echo ""

# List current backups
echo "Current backups:"
ls -lh "$BACKUP_DIR"/conduit_backup_*.tar.gz 2>/dev/null | tail -5
echo ""

# ========================================
# Restore instructions
# ========================================
echo -e "${YELLOW}To restore from this backup:${NC}"
echo "1. Stop Conduit: docker stop conduit"
echo "2. Remove old volume: docker volume rm ${DATA_VOLUME}"
echo "3. Create new volume: docker volume create ${DATA_VOLUME}"
echo "4. Extract backup:"
echo "   docker run --rm -v ${DATA_VOLUME}:/data -v ${BACKUP_DIR}:/backup alpine sh -c 'tar -xzf /backup/${BACKUP_NAME} -C /data'"
echo "5. Start Conduit: docker start conduit"
echo ""
