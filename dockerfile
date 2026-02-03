# ───── Stage 1: Builder ─────
FROM rust:latest AS builder

WORKDIR /usr/src/app

# Copy Cargo files for caching
COPY Cargo.toml Cargo.lock ./

# Temporary main.rs for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy the full Rust + Python project
COPY . .

# Instalsfsl Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Build Rust CLI
RUN cargo build --release

# ───── Stage 2: Runtime ─────
FROM python:3.11-slim

WORKDIR /data

# Copy Rust binary
COPY --from=builder /usr/src/app/target/release/vb /usr/local/bin/vb

# Copy Python scripts and requirements
COPY --from=builder /usr/src/app/extract_pdf.py /usr/local/bin/extract_pdf.py
COPY --from=builder /usr/src/app/requirements.txt /usr/local/bin/requirements.txt

# Install Python dependencies
RUN pip install --no-cache-dir -r /usr/local/bin/requirements.txt

# Make CLI executable
RUN chmod +x /usr/local/bin/vb

# Entrypoint is the Rust CLI
ENTRYPOINT ["vb"]
