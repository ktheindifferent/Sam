import os
import subprocess
import tempfile
from pathlib import Path

from flask import Flask, jsonify, request, send_file


app = Flask(__name__)


@app.get("/health")
def health():
    return jsonify({"status": "healthy", "engine": os.getenv("SAM_VOICE_ENGINE", "espeak")})


@app.post("/tts")
def tts():
    payload = request.get_json(silent=True) or {}
    text = str(payload.get("text") or request.form.get("text") or "").strip()
    if not text:
        return jsonify({"error": "text is required"}), 400

    language = str(payload.get("language") or os.getenv("SAM_VOICE_LANGUAGE", "en-us"))
    speed = str(payload.get("speed") or os.getenv("SAM_VOICE_SPEED", "175"))
    voice = str(payload.get("voice") or language)

    with tempfile.NamedTemporaryFile(prefix="sam_tts_", suffix=".wav", delete=False) as tmp:
        output_path = Path(tmp.name)

    try:
        subprocess.run(
            ["espeak-ng", "-v", voice, "-s", speed, "-w", str(output_path), text],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
        )
        return send_file(output_path, mimetype="audio/wav", as_attachment=False)
    except subprocess.CalledProcessError as exc:
        return jsonify({"error": "tts failed", "detail": exc.stderr.decode("utf-8", "ignore")}), 500
    except subprocess.TimeoutExpired:
        return jsonify({"error": "tts timed out"}), 504
    finally:
        try:
            output_path.unlink(missing_ok=True)
        except OSError:
            pass


@app.post("/stt")
def stt():
    return jsonify({
        "text": "",
        "confidence": 0.0,
        "language": None,
        "processing_time_ms": 0,
        "message": "STT container is reachable; whisper backend is not bundled in this lightweight CapRover voice image yet."
    })


if __name__ == "__main__":
    port = int(os.getenv("PORT", "8002"))
    app.run(host="0.0.0.0", port=port)
