# Blue Team Incident Response Playbook

This playbook provides structured procedures for blue team operations during red team attack cycles. It covers pre-attack preparation, during-attack monitoring, and post-attack analysis.

---

## Pre-Attack Checklist

Complete these steps before starting any red team attack cycle.

### 1. Verify Monitoring Infrastructure

- [ ] **Prometheus is running** — `curl http://localhost:9090/-/healthy` returns 200
- [ ] **Grafana is running** — `curl http://localhost:3000/api/health` returns 200
- [ ] **Blue team observer is connected** — check `opendex_observer_connected` metric = 1
- [ ] **All honest nodes are scraped** — verify 4 targets in Prometheus "UP" state
- [ ] **Alert rules are loaded** — check Prometheus `/api/v1/rules` for blue team rules

### 2. Establish Baselines

Record these metrics before attacks begin:

| Metric | Baseline Value |
|--------|---------------|
| `opendex_message_processed_total` (5min rate) | [Record] |
| `opendex_message_rejected_total` (5min rate) | [Record] |
| `opendex_connection_count` (avg) | [Record] |
| `opendex_memory_usage_bytes` (avg) | [Record] |
| `opendex_cpu_usage_percent` (avg) | [Record] |
| `opendex_auth_failure_total` (5min rate) | [Record] |

### 3. Verify Alert Thresholds

- [ ] `AttackDetected` threshold set to 10x baseline message rejection rate
- [ ] `AuthFailureSpike` threshold set to 10 failures/min
- [ ] `DoSSuspected` threshold set to 80% of max connections
- [ ] `MemoryPressure` threshold set to 80% of memory limit
- [ ] All alerts are in `INACTIVE` state (no false positives)

### 4. Prepare Documentation

- [ ] Create a new report from `blueteam/reports/template.md`
- [ ] Record the start time
- [ ] Note which red team scenarios will be executed
- [ ] Assign observers to each attack category

---

## During-Attack Procedures

Follow these procedures while red team attacks are executing.

### 1. Monitor Real-Time Dashboards

- **Primary:** Grafana Blue Team Dashboard — watch for alert triggers
- **Secondary:** Prometheus alert panel — check for firing alerts
- **Tertiary:** Node logs — watch for error/warning messages

### 2. Document Each Attack

For each red team attack executed:

1. **Record attack start time** — note the exact timestamp
2. **Observe defense response** — watch for:
   - Message rejections in logs
   - Prometheus metric changes
   - Alert firings
3. **Record detection time** — note when the first alert or metric change occurs
4. **Record defense outcome** — note whether the defense worked:
   - ✅ **PASS** — defense worked, attack was blocked
   - ❌ **FAIL** — defense failed, attack succeeded
   - ⚠️ **PARTIAL** — defense partially worked
   - ❓ **ERROR** — unable to determine outcome

### 3. Alert Response Workflow

When an alert fires:

1. **Acknowledge the alert** — note the alert name and firing time
2. **Correlate with attack** — match the alert to the running red team scenario
3. **Verify the alert is valid** — check metrics to confirm the attack signature
4. **Document the detection** — record in the report:
   - Alert name
   - Firing time
   - Corresponding attack scenario
   - Detection latency (time from attack start to alert)

### 4. Handle Unexpected Events

If something unexpected occurs:

1. **Node crash** — record which node, when, and what attack was running
2. **False positive alert** — record the alert, investigate, and note the cause
3. **Missed detection** — if an attack runs but no alert fires, document immediately
4. **Observer disconnect** — note the time and attempt reconnection

### 5. Communication Protocol

- **Critical finding:** Notify immediately — attack succeeded with no detection
- **High finding:** Note for post-attack review — defense gap identified
- **Medium finding:** Record in report — minor detection delay or partial defense
- **Low finding:** Log for trend analysis — informational observation

---

## Post-Attack Analysis

Complete these steps after all red team attacks are finished.

### 1. Collect Final Metrics

| Metric | Final Value | Change from Baseline |
|--------|------------|---------------------|
| `opendex_message_processed_total` | [Record] | [Calculate] |
| `opendex_message_rejected_total` | [Record] | [Calculate] |
| `opendex_message_replay_detected_total` | [Record] | [Calculate] |
| `opendex_auth_failure_total` | [Record] | [Calculate] |
| `opendex_rate_limiter_activations_total` | [Record] | [Calculate] |
| `opendex_connection_rejected_total` | [Record] | [Calculate] |

### 2. Compare Red Team vs Blue Team Results

For each red team scenario:

| Red Team Scenario | Blue Team Validation | Red Result | Blue Result | Match? |
|-------------------|---------------------|------------|-------------|--------|
| PA-001 | BD-001 | [Result] | [Result] | ✅/❌ |
| PA-002 | BD-002 | [Result] | [Result] | ✅/❌ |

### 3. Calculate Detection Metrics

- **Total attacks executed:** [X]
- **Total attacks detected:** [X]
- **Detection rate:** [X]%
- **Average detection time:** [X] seconds
- **Alerts fired:** [X]
- **False positive alerts:** [X]
- **Missed detections:** [X]

### 4. Identify Defense Gaps

List all attacks that were NOT detected:

1. **[Attack ID]:** [Description of what was missed and why]
2. **[Attack ID]:** [Description]

### 5. Generate Report

1. Fill in the blue team report template with all collected data
2. Include Grafana screenshots for key findings
3. Attach relevant log files
4. Compare with red team report findings
5. List recommendations for defense improvements

### 6. Follow-Up Actions

- [ ] **Fix critical gaps** — prioritize attacks that succeeded with no detection
- [ ] **Update alert thresholds** — adjust based on false positive/negative analysis
- [ ] **Add missing metrics** — if detection required manual log checking, add Prometheus metrics
- [ ] **Add regression tests** — create tests for any newly discovered vulnerabilities
- [ ] **Update threat model** — incorporate findings into `audit/threat_model.md`
- [ ] **Schedule re-test** — plan follow-up test for fixed gaps

---

## Escalation Matrix

| Severity | Condition | Response Time | Escalation |
|----------|-----------|--------------|------------|
| **Critical** | Attack succeeded, no detection, data at risk | Immediate | Security lead + engineering |
| **High** | Defense gap found, attack partially succeeded | < 1 hour | Security team |
| **Medium** | Detection delay > 30 seconds | < 4 hours | Monitoring team |
| **Low** | False positive alert, minor issue | Next review | On-call observer |

---

## Quick Reference

### Key Prometheus Queries

```promql
# Message rejection rate (should spike during attacks)
rate(opendex_message_rejected_total[5m])

# Replay detection count
increase(opendex_message_replay_detected_total[1h])

# Auth failure rate per peer
rate(opendex_auth_failure_total[5m])

# Connection count vs. limit
opendex_connection_count / opendex_connection_max

# Memory usage percentage
opendex_memory_usage_bytes / opendex_memory_limit_bytes * 100

# State machine invalid transitions
increase(opendex_fiat_invalid_transition_total[1h])
```

### Key Grafana Panels

1. **Attack Detection Timeline** — shows attacks vs. detections
2. **Defense Health Score** — composite pass/fail rate
3. **Message Flow Analysis** — valid vs. rejected messages
4. **Authentication Security** — auth failures and rate limiting
5. **Resource Protection** — memory, CPU, connections
6. **Fiat Trade Security** — payment verification and reversals

### Common Issues

| Issue | Cause | Fix |
|-------|-------|-----|
| Observer not connecting | Wrong peer address | Verify `--peer-addr` CLI argument |
| Alerts not firing | Threshold too high | Adjust based on baseline metrics |
| Missing metrics | Node not exposing `/metrics` | Check node configuration |
| Grafana dashboard empty | Data source not configured | Verify Prometheus data source |
| High false positive rate | Thresholds too low | Increase thresholds based on baseline |
