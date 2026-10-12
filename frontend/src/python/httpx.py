import json

import js
from pyodide.ffi import JsException, run_sync


class ConnectError(Exception):
    pass


class URL:
    def __init__(self, url):
        self.host = "127.0.0.1"


class Response:
    def __init__(self, status_code, body):
        self.status_code = status_code
        self.text = body

    def json(self):
        return json.loads(self.text)


def post(url, json=None, timeout=None, trust_env=None):
    import json as _json
    payload = json or {}
    try:
        text = run_sync(js.formsurfaceAgent(payload.get("command", ""), _json.dumps(payload.get("args", {}))))
    except JsException as error:
        return Response(400, _json.dumps({"detail": str(error).split("Error: ", 1)[-1]}))
    return Response(200, _json.dumps({"result": _json.loads(text)}))
