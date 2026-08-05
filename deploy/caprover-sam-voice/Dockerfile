FROM python:3.12-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    espeak-ng \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY server.py .

ENV PORT=8002 \
    SAM_VOICE_ENGINE=espeak \
    SAM_VOICE_LANGUAGE=en-us

EXPOSE 8002

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS "http://localhost:${PORT}/health" >/dev/null || exit 1

CMD ["python", "server.py"]
