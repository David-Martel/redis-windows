# Agent Sync - Production Deployment Guide

This guide covers deploying Agent Sync in production environments, including Windows servers, Linux systems, WSL, Docker, and Kubernetes.

## Table of Contents

- [Pre-Deployment Checklist](#pre-deployment-checklist)
- [System Requirements](#system-requirements)
- [Windows Deployment](#windows-deployment)
- [Linux Deployment](#linux-deployment)
- [WSL Deployment](#wsl-deployment)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Redis Configuration](#redis-configuration)
- [Security Hardening](#security-hardening)
- [Monitoring and Observability](#monitoring-and-observability)
- [High Availability](#high-availability)
- [Backup and Recovery](#backup-and-recovery)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)

## Pre-Deployment Checklist

### Essential Requirements

- [ ] Redis 6.0+ deployed and accessible
- [ ] TLS certificates configured (production)
- [ ] Authentication credentials secured
- [ ] Firewall rules configured
- [ ] Monitoring infrastructure ready
- [ ] Backup strategy defined
- [ ] Load testing completed
- [ ] Security audit passed

### Recommended Setup

- [ ] Redis Cluster or Sentinel for HA
- [ ] Connection pooling configured
- [ ] Rate limiting implemented
- [ ] Circuit breakers enabled
- [ ] Distributed tracing setup
- [ ] Log aggregation configured
- [ ] Alerting rules defined
- [ ] Runbook documented

## System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| CPU | 2 cores |
| RAM | 4 GB |
| Disk | 10 GB SSD |
| Network | 1 Gbps |
| OS | Windows Server 2019+ / Linux Kernel 4.15+ |
| Redis | 6.0+ |

### Recommended Production Setup

| Component | Requirement |
|-----------|-------------|
| CPU | 8+ cores |
| RAM | 16-32 GB |
| Disk | 100 GB NVMe SSD |
| Network | 10 Gbps |
| OS | Windows Server 2022 / Ubuntu 22.04 LTS |
| Redis | 7.0+ Cluster |

## Windows Deployment

### Windows Server Setup

#### 1. Prerequisites

```powershell
# Install Visual C++ Runtime
Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vc_redist.x64.exe" -OutFile "vc_redist.x64.exe"
.\vc_redist.x64.exe /install /quiet /norestart

# Install .NET Runtime (if using .NET integration)
winget install Microsoft.DotNet.Runtime.6

# Configure Windows Defender exclusions
Add-MpPreference -ExclusionPath "C:\AgentSync"
Add-MpPreference -ExclusionProcess "agent-sync.exe"
```

#### 2. Installation

```powershell
# Create installation directory
New-Item -ItemType Directory -Path "C:\AgentSync" -Force
Set-Location "C:\AgentSync"

# Download and extract release
$version = "0.1.0"
$url = "https://github.com/anthropic/redis-windows/releases/download/v$version/agent-sync-windows-x64.zip"
Invoke-WebRequest -Uri $url -OutFile "agent-sync.zip"
Expand-Archive -Path "agent-sync.zip" -DestinationPath "." -Force

# Set permissions
$acl = Get-Acl "C:\AgentSync"
$permission = "NT SERVICE\AgentSync","FullControl","Allow"
$accessRule = New-Object System.Security.AccessControl.FileSystemAccessRule $permission
$acl.SetAccessRule($accessRule)
Set-Acl "C:\AgentSync" $acl
```

#### 3. Windows Service Configuration

Create Windows Service using PowerShell:

```powershell
# Install as Windows Service
New-Service -Name "AgentSync" `
    -BinaryPathName "C:\AgentSync\agent-sync.exe --config C:\AgentSync\config.toml" `
    -DisplayName "Agent Sync Service" `
    -Description "High-performance Redis agent synchronization service" `
    -StartupType Automatic `
    -Credential (Get-Credential)

# Configure service recovery
sc.exe failure AgentSync reset=86400 actions=restart/5000/restart/10000/restart/30000

# Configure service dependencies
sc.exe config AgentSync depend=Tcpip/Afd/Redis
```

#### 4. Configuration File

Create `C:\AgentSync\config.toml`:

```toml
[service]
name = "agent-sync-prod"
log_level = "info"
log_file = "C:\\AgentSync\\logs\\agent-sync.log"

[redis]
url = "redis://redis-server.company.local:6379"
password = "${REDIS_PASSWORD}"  # Use environment variable
db = 0
connection_timeout_ms = 5000
command_timeout_ms = 3000

[pool]
max_size = 100
min_idle = 20
max_lifetime_ms = 1800000  # 30 minutes

[security]
tls_enabled = true
tls_cert_path = "C:\\AgentSync\\certs\\client.crt"
tls_key_path = "C:\\AgentSync\\certs\\client.key"
tls_ca_path = "C:\\AgentSync\\certs\\ca.crt"

[metrics]
enabled = true
port = 9090
path = "/metrics"

[health]
enabled = true
port = 8080
path = "/health"
```

#### 5. Start Service

```powershell
# Start the service
Start-Service -Name AgentSync

# Verify status
Get-Service -Name AgentSync

# Check logs
Get-Content "C:\AgentSync\logs\agent-sync.log" -Tail 50 -Wait
```

### IIS Integration (Optional)

For web-based agent coordination:

```xml
<!-- web.config -->
<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <system.webServer>
    <handlers>
      <add name="AgentSyncProxy" path="*" verb="*"
           type="AgentSync.Proxy.Handler" resourceType="Unspecified" />
    </handlers>
    <httpPlatform processPath="C:\AgentSync\agent-sync.exe"
                  arguments="--web-server --port %HTTP_PLATFORM_PORT%"
                  stdoutLogEnabled="true"
                  stdoutLogFile="C:\AgentSync\logs\stdout.log">
      <environmentVariables>
        <environmentVariable name="REDIS_PASSWORD" value="ENCRYPTED_VALUE" />
      </environmentVariables>
    </httpPlatform>
  </system.webServer>
</configuration>
```

## Linux Deployment

### Ubuntu/Debian Setup

#### 1. Installation

```bash
# Create application user
sudo useradd -r -s /bin/false agent-sync

# Create directories
sudo mkdir -p /opt/agent-sync/{bin,config,logs,data}
sudo chown -R agent-sync:agent-sync /opt/agent-sync

# Download and install
VERSION="0.1.0"
wget -O /tmp/agent-sync.tar.gz \
  "https://github.com/anthropic/redis-windows/releases/download/v${VERSION}/agent-sync-linux-x64.tar.gz"

sudo tar -xzf /tmp/agent-sync.tar.gz -C /opt/agent-sync/bin/
sudo chmod +x /opt/agent-sync/bin/agent-sync
```

#### 2. Systemd Service

Create `/etc/systemd/system/agent-sync.service`:

```ini
[Unit]
Description=Agent Sync Service
Documentation=https://github.com/anthropic/redis-windows
After=network.target redis.service
Wants=redis.service

[Service]
Type=simple
User=agent-sync
Group=agent-sync
WorkingDirectory=/opt/agent-sync

# Environment
Environment="RUST_LOG=info"
Environment="REDIS_URL=redis://localhost:6379"
EnvironmentFile=-/etc/agent-sync/environment

# Execution
ExecStart=/opt/agent-sync/bin/agent-sync --config /opt/agent-sync/config/production.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=10

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/agent-sync/logs /opt/agent-sync/data

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096
MemoryLimit=2G
CPUQuota=200%

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=agent-sync

[Install]
WantedBy=multi-user.target
```

#### 3. Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable and start service
sudo systemctl enable agent-sync
sudo systemctl start agent-sync

# Check status
sudo systemctl status agent-sync

# View logs
sudo journalctl -u agent-sync -f
```

### RHEL/CentOS Setup

Similar to Ubuntu with adjustments:

```bash
# Install dependencies
sudo yum install -y epel-release
sudo yum install -y redis supervisor

# SELinux configuration
sudo setsebool -P httpd_can_network_connect 1
sudo semanage port -a -t http_port_t -p tcp 9090

# Firewall configuration
sudo firewall-cmd --permanent --add-port=9090/tcp
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --reload
```

## WSL Deployment

### WSL2 Optimization

#### 1. WSL Configuration

Create `.wslconfig` in Windows user home:

```ini
[wsl2]
memory=8GB
processors=4
swap=2GB
localhostForwarding=true

[experimental]
autoMemoryReclaim=gradual
networkingMode=mirrored
dnsTunneling=true
```

#### 2. Installation Script

```bash
#!/bin/bash
# install-wsl.sh

# Detect WSL
if ! grep -q Microsoft /proc/version; then
    echo "Not running in WSL"
    exit 1
fi

# Install in WSL
sudo apt-get update
sudo apt-get install -y build-essential redis-server

# Download and install agent-sync
VERSION="0.1.0"
wget -O /tmp/agent-sync.tar.gz \
  "https://github.com/anthropic/redis-windows/releases/download/v${VERSION}/agent-sync-linux-x64.tar.gz"

sudo mkdir -p /opt/agent-sync
sudo tar -xzf /tmp/agent-sync.tar.gz -C /opt/agent-sync/

# Configure for WSL
cat > /opt/agent-sync/wsl-config.toml << EOF
[wsl]
enabled = true
windows_interop = true
share_windows_redis = true

[redis]
# Use Windows Redis if available
url = "redis://host.docker.internal:6379"
fallback_url = "redis://localhost:6379"
EOF

# Start service
/opt/agent-sync/agent-sync --config /opt/agent-sync/wsl-config.toml
```

## Docker Deployment

### Production Dockerfile

```dockerfile
# Multi-stage build for production
FROM rust:1.75 AS builder

WORKDIR /build
COPY . .

# Build with optimizations
RUN cargo build --release --features "metrics tls cluster"

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -u 1000 -s /bin/false agent-sync

# Copy binary
COPY --from=builder /build/target/release/agent-sync /usr/local/bin/

# Configuration
COPY docker/config.toml /etc/agent-sync/config.toml

# Switch to non-root user
USER agent-sync

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/agent-sync", "health"]

# Metrics port
EXPOSE 9090

# Health check port
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/agent-sync"]
CMD ["--config", "/etc/agent-sync/config.toml"]
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  agent-sync:
    image: agent-sync:latest
    container_name: agent-sync-prod
    restart: always
    ports:
      - "9090:9090"  # Metrics
      - "8080:8080"  # Health
    environment:
      - RUST_LOG=info
      - REDIS_URL=redis://redis:6379
      - REDIS_PASSWORD=${REDIS_PASSWORD}
    volumes:
      - ./config:/etc/agent-sync
      - agent-sync-data:/data
    networks:
      - agent-network
    depends_on:
      redis:
        condition: service_healthy
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G

  redis:
    image: redis:7-alpine
    container_name: redis-prod
    restart: always
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
      - ./redis.conf:/usr/local/etc/redis/redis.conf
    command: redis-server /usr/local/etc/redis/redis.conf
    networks:
      - agent-network
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    networks:
      - agent-network

volumes:
  agent-sync-data:
  redis-data:
  prometheus-data:

networks:
  agent-network:
    driver: bridge
```

## Kubernetes Deployment

### Kubernetes Manifests

#### Deployment

```yaml
# agent-sync-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: agent-sync
  namespace: production
  labels:
    app: agent-sync
    version: v0.1.0
spec:
  replicas: 3
  selector:
    matchLabels:
      app: agent-sync
  template:
    metadata:
      labels:
        app: agent-sync
        version: v0.1.0
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: agent-sync
      containers:
      - name: agent-sync
        image: agent-sync:0.1.0
        imagePullPolicy: Always
        ports:
        - containerPort: 9090
          name: metrics
        - containerPort: 8080
          name: health
        env:
        - name: RUST_LOG
          value: "info"
        - name: REDIS_URL
          value: "redis://redis-service:6379"
        - name: REDIS_PASSWORD
          valueFrom:
            secretKeyRef:
              name: redis-credentials
              key: password
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        volumeMounts:
        - name: config
          mountPath: /etc/agent-sync
          readOnly: true
      volumes:
      - name: config
        configMap:
          name: agent-sync-config
```

#### Service

```yaml
# agent-sync-service.yaml
apiVersion: v1
kind: Service
metadata:
  name: agent-sync-service
  namespace: production
  labels:
    app: agent-sync
spec:
  type: ClusterIP
  ports:
  - port: 9090
    targetPort: 9090
    protocol: TCP
    name: metrics
  - port: 8080
    targetPort: 8080
    protocol: TCP
    name: health
  selector:
    app: agent-sync
```

#### ConfigMap

```yaml
# agent-sync-configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: agent-sync-config
  namespace: production
data:
  config.toml: |
    [redis]
    url = "redis://redis-service:6379"
    db = 0
    pool_size = 50

    [metrics]
    enabled = true
    port = 9090

    [health]
    enabled = true
    port = 8080
```

#### HPA (Horizontal Pod Autoscaler)

```yaml
# agent-sync-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: agent-sync-hpa
  namespace: production
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: agent-sync
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Helm Chart

```yaml
# values.yaml
replicaCount: 3

image:
  repository: agent-sync
  tag: "0.1.0"
  pullPolicy: IfNotPresent

service:
  type: ClusterIP
  metricsPort: 9090
  healthPort: 8080

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: agent-sync.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: agent-sync-tls
      hosts:
        - agent-sync.example.com

resources:
  limits:
    cpu: 1000m
    memory: 2Gi
  requests:
    cpu: 250m
    memory: 512Mi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

redis:
  enabled: true
  architecture: replication
  auth:
    enabled: true
    password: "changeme"
  master:
    persistence:
      enabled: true
      size: 10Gi
  replica:
    replicaCount: 2
    persistence:
      enabled: true
      size: 10Gi

monitoring:
  enabled: true
  serviceMonitor:
    enabled: true
    interval: 30s
```

## Redis Configuration

### Production Redis Settings

```conf
# redis-production.conf

# Network
bind 0.0.0.0
protected-mode yes
port 6379
tcp-backlog 511
tcp-keepalive 300
timeout 0

# General
daemonize no
supervised systemd
loglevel notice
logfile /var/log/redis/redis-server.log
databases 16

# Memory Management
maxmemory 8gb
maxmemory-policy allkeys-lru
maxmemory-samples 5

# Persistence
save 900 1
save 300 10
save 60 10000
stop-writes-on-bgsave-error yes
rdbcompression yes
rdbchecksum yes
dbfilename dump.rdb
dir /var/lib/redis

# AOF
appendonly yes
appendfilename "appendonly.aof"
appendfsync everysec
no-appendfsync-on-rewrite no
auto-aof-rewrite-percentage 100
auto-aof-rewrite-min-size 64mb

# Replication
replicaof no one
replica-serve-stale-data yes
replica-read-only yes
repl-diskless-sync no
repl-diskless-sync-delay 5

# Security
requirepass ${REDIS_PASSWORD}
masterauth ${REDIS_PASSWORD}

# Limits
slowlog-log-slower-than 10000
slowlog-max-len 128

# Advanced
hash-max-ziplist-entries 512
hash-max-ziplist-value 64
list-max-ziplist-size -2
list-compress-depth 0
set-max-intset-entries 512
zset-max-ziplist-entries 128
zset-max-ziplist-value 64

# Modules
loadmodule /usr/lib/redis/modules/redisearch.so
loadmodule /usr/lib/redis/modules/redisjson.so

# Stream settings for agent coordination
stream-node-max-bytes 4096
stream-node-max-entries 100
```

### Redis Cluster Configuration

```bash
# Create cluster
redis-cli --cluster create \
  127.0.0.1:7000 127.0.0.1:7001 127.0.0.1:7002 \
  127.0.0.1:7003 127.0.0.1:7004 127.0.0.1:7005 \
  --cluster-replicas 1 \
  --cluster-yes

# Check cluster status
redis-cli --cluster check 127.0.0.1:7000
```

## Security Hardening

### TLS Configuration

```toml
# tls-config.toml
[security.tls]
enabled = true
cert_path = "/etc/agent-sync/certs/server.crt"
key_path = "/etc/agent-sync/certs/server.key"
ca_path = "/etc/agent-sync/certs/ca.crt"
verify_mode = "peer"
min_version = "TLS1.2"
cipher_suites = [
    "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
    "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384",
    "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305",
    "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305"
]
```

### Authentication & Authorization

```rust
// Custom authentication middleware
use agent_sync::auth::{AuthMiddleware, JwtValidator};

let auth = AuthMiddleware::new()
    .with_jwt_validator(JwtValidator::from_secret("secret"))
    .with_api_key_validator(|key| {
        // Validate API key
        database.validate_api_key(key)
    })
    .with_rate_limiter(100, Duration::from_secs(60));

app.layer(auth);
```

### Network Security

```bash
# UFW firewall rules (Ubuntu)
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow from 10.0.0.0/8 to any port 6379  # Redis
sudo ufw allow from 10.0.0.0/8 to any port 9090  # Metrics
sudo ufw allow 443/tcp  # HTTPS
sudo ufw enable

# iptables rules
iptables -A INPUT -p tcp --dport 6379 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 6379 -j DROP
```

## Monitoring and Observability

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'agent-sync'
    static_configs:
      - targets: ['agent-sync:9090']
    metrics_path: '/metrics'

  - job_name: 'redis'
    static_configs:
      - targets: ['redis:9121']

rule_files:
  - 'alerts.yml'
```

### Alert Rules

```yaml
# alerts.yml
groups:
  - name: agent_sync_alerts
    interval: 30s
    rules:
      - alert: HighTaskQueueDepth
        expr: agent_sync_task_queue_depth > 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High task queue depth"
          description: "Task queue depth is {{ $value }} on {{ $labels.instance }}"

      - alert: HighErrorRate
        expr: rate(agent_sync_errors_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate"
          description: "Error rate is {{ $value }} on {{ $labels.instance }}"

      - alert: RedisConnectionFailure
        expr: agent_sync_redis_connections_active == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Redis connection failure"
          description: "No active Redis connections on {{ $labels.instance }}"
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "Agent Sync Monitoring",
    "panels": [
      {
        "title": "Task Processing Rate",
        "targets": [
          {
            "expr": "rate(agent_sync_tasks_processed_total[5m])"
          }
        ]
      },
      {
        "title": "Queue Depth",
        "targets": [
          {
            "expr": "agent_sync_task_queue_depth"
          }
        ]
      },
      {
        "title": "Redis Connections",
        "targets": [
          {
            "expr": "agent_sync_redis_connections_active"
          }
        ]
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(agent_sync_errors_total[5m])"
          }
        ]
      }
    ]
  }
}
```

### Logging Configuration

```toml
# logging.toml
[logging]
level = "info"
format = "json"
output = "stdout"

[logging.file]
enabled = true
path = "/var/log/agent-sync/agent-sync.log"
max_size_mb = 100
max_backups = 10
max_age_days = 30
compress = true

[logging.syslog]
enabled = true
address = "syslog-server:514"
facility = "local0"
tag = "agent-sync"

[logging.fields]
service = "agent-sync"
environment = "production"
version = "0.1.0"
```

## High Availability

### Multi-Region Deployment

```yaml
# Global load balancer configuration
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: agent-sync-global
  annotations:
    kubernetes.io/ingress.class: "nginx"
    nginx.ingress.kubernetes.io/backend-protocol: "HTTP"
    nginx.ingress.kubernetes.io/load-balance: "least_conn"
spec:
  rules:
  - host: agent-sync.global.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: agent-sync-service
            port:
              number: 8080
```

### Disaster Recovery

```bash
#!/bin/bash
# disaster-recovery.sh

# Backup Redis data
redis-cli --rdb /backup/redis-$(date +%Y%m%d).rdb

# Backup agent-sync configuration
tar -czf /backup/agent-sync-config-$(date +%Y%m%d).tar.gz /etc/agent-sync/

# Sync to remote storage
aws s3 sync /backup/ s3://backup-bucket/agent-sync/

# Test restore procedure
docker run -d --name redis-restore redis:7
docker cp /backup/redis-latest.rdb redis-restore:/data/dump.rdb
docker restart redis-restore
```

## Performance Tuning

### System Tuning

```bash
# Linux kernel parameters
cat >> /etc/sysctl.conf << EOF
# Network
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.ip_local_port_range = 1024 65535
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 30

# Memory
vm.overcommit_memory = 1
vm.swappiness = 10

# File descriptors
fs.file-max = 2097152
EOF

sysctl -p

# User limits
cat >> /etc/security/limits.conf << EOF
agent-sync soft nofile 65535
agent-sync hard nofile 65535
agent-sync soft nproc 32768
agent-sync hard nproc 32768
EOF
```

### Application Tuning

```toml
# performance.toml
[performance]
# Connection pool
pool_size = 100
pool_min_idle = 20
connection_timeout_ms = 5000

# Batching
batch_size = 1000
batch_timeout_ms = 100

# Caching
cache_enabled = true
cache_size_mb = 512
cache_ttl_seconds = 300

# Compression
compression_enabled = true
compression_threshold_bytes = 1024
compression_level = 6

# Threading
worker_threads = 16
async_threads = 8
blocking_threads = 512
```

## Troubleshooting

### Common Issues

#### Redis Connection Issues

```bash
# Test Redis connectivity
redis-cli -h redis-server -p 6379 ping

# Check Redis logs
tail -f /var/log/redis/redis-server.log

# Monitor Redis commands
redis-cli monitor

# Check Redis info
redis-cli info stats
```

#### High Memory Usage

```bash
# Check memory usage
ps aux | grep agent-sync
top -p $(pgrep agent-sync)

# Generate heap dump
kill -USR1 $(pgrep agent-sync)

# Analyze with valgrind
valgrind --leak-check=full --show-leak-kinds=all ./agent-sync
```

#### Performance Issues

```bash
# CPU profiling
perf record -F 99 -p $(pgrep agent-sync) -g -- sleep 30
perf report

# Trace system calls
strace -c -p $(pgrep agent-sync)

# Network analysis
tcpdump -i any -w capture.pcap host redis-server
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug,agent_sync=trace ./agent-sync

# Enable backtrace
RUST_BACKTRACE=full ./agent-sync

# Enable memory tracking
MALLOC_STATS=1 ./agent-sync
```

### Health Checks

```bash
# Check service health
curl http://localhost:8080/health

# Check metrics
curl http://localhost:9090/metrics | grep agent_sync

# Check Redis health
redis-cli ping
redis-cli --latency
redis-cli --bigkeys
```

## Maintenance

### Rolling Updates

```bash
# Kubernetes rolling update
kubectl set image deployment/agent-sync agent-sync=agent-sync:0.2.0 -n production
kubectl rollout status deployment/agent-sync -n production

# Docker Swarm update
docker service update --image agent-sync:0.2.0 agent-sync

# Manual rolling update
for server in server1 server2 server3; do
    ssh $server "systemctl stop agent-sync"
    ssh $server "apt-get update && apt-get install agent-sync"
    ssh $server "systemctl start agent-sync"
    sleep 30
done
```

### Backup Procedures

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backup/$(date +%Y%m%d)"
mkdir -p $BACKUP_DIR

# Backup Redis
redis-cli BGSAVE
sleep 10
cp /var/lib/redis/dump.rdb $BACKUP_DIR/

# Backup configuration
cp -r /etc/agent-sync $BACKUP_DIR/

# Backup logs
tar -czf $BACKUP_DIR/logs.tar.gz /var/log/agent-sync/

# Upload to S3
aws s3 cp $BACKUP_DIR s3://backups/agent-sync/ --recursive
```

## Support

For production support:

- **Documentation**: [https://docs.agent-sync.io](https://docs.agent-sync.io)
- **Enterprise Support**: support@anthropic.com
- **Security Issues**: security@anthropic.com
- **24/7 Hotline**: +1-XXX-XXX-XXXX (Enterprise customers only)