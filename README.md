# dtqs

## Overview

DTQS is a scalable distributed task-queue platform designed to decouple task submission from processing. It leverages RabbitMQ for messaging, PostgreSQL for persistence, and Rust-based microservices to handle high-throughput asynchronous workloads (e.g., email/SMS notifications, image/video processing). 
DTQS ensures reliability, fault tolerance, and real-time monitoring via a CLI dashboard.

## Features

- **Decoupled Architecture**: Separate API server for task intake, RabbitMQ broker for queuing, and worker nodes for processing.
- **Persistent Task Metadata**: All tasks are stored in PostgreSQL with UUIDs, payloads, status, priority, and timestamps for auditability.
- **Asynchronous Processing**: Workers consume tasks from RabbitMQ, update status in PostgreSQL, and support retry-on-failure with exponential backoff.
- **Real-Time Streaming**: Server-Sent Events endpoint streams task outcomes to clients.
- **CLI Dashboard**: TUI-based interface to monitor worker statuses, pending tasks, and logs with live refresh.
- **Containerized & Orchestrated**: Dockerized services with Kubernetes manifests for on-demand scaling.

## Architecture

- **API Server**
    - **Endpoints**
        - `POST /submit`: Validate JSON payload, insert metadata into PostgreSQL, enqueue message to RabbitMQ.
        - `GET /sse`: Open SSE connection and stream final task result once status is “completed” or “failed.”

- **RabbitMQ Broker**
    - Single `task_queue` with priority support.
    - API Server publishes tasks after insertion; Worker Nodes consume messages for processing.

- **Worker Nodes**
    - **Consumer Loop**: Consume tasks, retrieve metadata from PostgreSQL, process based on `task_type`.
    - **Task Types**: Email, image processing, video encoding, etc.
    - **Progress Logging**: Append periodic status logs to `logs` table; update task status (`pending` → `in_progress` → `completed`/`failed`).
    - **Resilience Pipeline**: Exponential backoff up to 5 retries on transient failures; requeue or mark as failed.

- **CLI Dashboard**
    - Built with `tui` + `crossterm`.
    - Polls PostgreSQL and RabbitMQ every 2 seconds.
    - Displays:
        - **Overview Tab**: Active worker nodes and their status.
        - **Queue Tab**: Next 5 pending tasks (ID, type, priority, enqueued time).
        - **Logs Tab**: Recent log entries with timestamps.
