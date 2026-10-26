import json

import pytest

PNG = b"\x89PNG\r\n\x1a\n" + b"\x00" * 16
TINY_PNG = "data:image/png;base64,iVBORw0KGgo="


def upload(client, name, filename, data, field="maps", path="/api/exports"):
    return client.post(f"{path}/{name}", files=[(field, (filename, data, "application/octet-stream"))])


def test_export_writes_a_map_whose_bytes_match_its_format(client, data_dir):
    r = upload(client, "car", "car_basecolor.png", PNG)
    assert r.status_code == 200
    assert r.json()["files"] == ["car_basecolor.png"]
    assert (data_dir / "exports" / "car" / "car_basecolor.png").read_bytes() == PNG


@pytest.mark.parametrize("filename,data", [
    ("map.png", b"not a png at all"),
    ("map.exe", PNG),
    ("map.png.txt", PNG),
    ("map.exr", PNG),
])
def test_export_refuses_a_map_that_is_not_what_its_name_says(client, filename, data):
    assert upload(client, "refused", filename, data).status_code == 400


def test_export_keeps_only_the_file_name_of_an_uploaded_path(client, data_dir):
    r = upload(client, "paths", "../../escape.png", PNG)
    assert r.status_code == 200
    assert r.json()["files"] == ["escape.png"]
    assert not (data_dir / "escape.png").exists()


@pytest.mark.parametrize("name", ["..hidden", ".dot", "a" * 81, "bad$name"])
def test_export_refuses_unsafe_folder_names(client, name):
    assert upload(client, name, "a.png", PNG).status_code == 400


def test_mesh_export_needs_an_obj_written_by_formsurface(client, data_dir):
    good = client.post("/api/exports/meshes/mesh", files=[("mesh", ("m.obj", b"# FormSurface\nv 0 0 0\n", "text/plain"))])
    assert good.status_code == 200
    assert (data_dir / "exports" / "meshes" / "m.obj").is_file()
    assert client.post("/api/exports/meshes/mesh", files=[("mesh", ("m.obj", b"v 0 0 0\n", "text/plain"))]).status_code == 400
    assert client.post("/api/exports/meshes/mesh", files=[("mesh", ("m.fbx", b"# FormSurface\n", "text/plain"))]).status_code == 400


def test_project_file_round_trips_byte_for_byte(client):
    blob = b"FSPJ" + bytes(range(256)) * 8
    saved = client.post("/api/projects/rt/file", content=blob)
    assert saved.status_code == 200
    assert saved.json()["bytes"] == len(blob)
    assert client.get("/api/projects/rt/file").content == blob
    assert "rt" in client.get("/api/projects").json()["projects"]


def test_project_file_must_carry_the_project_signature(client, data_dir):
    assert client.post("/api/projects/bad/file", content=b"XXXXjunk").status_code == 400
    assert not (data_dir / "projects" / "bad" / "project.fsproj").exists()


def test_project_file_save_leaves_no_partial_file(client, data_dir):
    client.post("/api/projects/clean/file", content=b"FSPJ1")
    assert sorted(p.name for p in (data_dir / "projects" / "clean").iterdir()) == ["project.fsproj"]


def test_project_document_is_stamped_with_the_version_and_read_back(client):
    r = client.post("/api/projects/doc", data={"document": json.dumps({"layers": [1, 2]})}, files=[("maps", ("a.png", PNG, "image/png"))])
    assert r.status_code == 200
    body = client.get("/api/projects/doc").json()
    assert body["document"]["layers"] == [1, 2]
    assert body["document"]["formsurface"] == client.get("/api/health").json()["version"]
    assert body["maps"] == ["a.png"]


def test_project_document_must_be_json(client):
    r = client.post("/api/projects/nojson", data={"document": "{broken"}, files=[("maps", ("a.png", PNG, "image/png"))])
    assert r.status_code == 400


def test_unknown_project_is_404(client):
    assert client.get("/api/projects/never-saved").status_code == 404
    assert client.get("/api/projects/never-saved/file").status_code == 404


def test_library_material_round_trips_and_is_validated(client):
    layers = [{"kind": "fill", "name": "rust"}]
    assert client.post("/api/library/rusty", json={"layers": layers, "thumbnail": TINY_PNG}).status_code == 200
    found = [m for m in client.get("/api/library").json()["materials"] if m["name"] == "rusty"]
    assert found[0]["layers"] == layers
    assert found[0]["thumbnail"] == TINY_PNG
    assert client.post("/api/library/empty", json={"layers": []}).status_code == 400
    assert client.post("/api/library/many", json={"layers": [{}] * 65}).status_code == 400
    assert client.post("/api/library/thumb", json={"layers": layers, "thumbnail": "data:text/html;base64,AAAA"}).status_code == 400
    assert client.post("/api/library/.dot", json={"layers": layers}).status_code == 400


def test_templates_keep_only_known_sources(client):
    good = {"label": "Pack", "maps": [{"suffix": "ORM", "r": "ao", "g": "roughness", "b": "metallic"}, {"suffix": "BC", "rgb": "basecolor"}]}
    assert client.post("/api/templates/pack", json=good).status_code == 200
    saved = [t for t in client.get("/api/templates").json()["templates"] if t["name"] == "pack"][0]
    assert saved["maps"][0] == {"suffix": "ORM", "r": "ao", "g": "roughness", "b": "metallic"}


@pytest.mark.parametrize("maps", [
    [],
    [{"suffix": "bad suffix", "rgb": "basecolor"}],
    [{"suffix": "X", "rgb": "unknown_source"}],
    [{"suffix": "X", "rgb": "ao"}],
    [{"suffix": "X", "r": "diffuse"}],
    [{"suffix": "X", "g": "ao"}],
])
def test_templates_refuse_bad_maps(client, maps):
    assert client.post("/api/templates/badpack", json={"maps": maps}).status_code == 400


def test_smart_mask_keeps_only_known_settings(client):
    body = {"settings": {"generator": "cavity", "generator_level": 0.4, "evil": "x", "generator_invert": 1}}
    assert client.post("/api/smart-masks/worn", json=body).status_code == 200
    saved = [m for m in client.get("/api/smart-masks").json()["masks"] if m["name"] == "worn"][0]
    assert saved["settings"] == {"generator": "cavity", "generator_level": 0.4, "generator_invert": True}
    assert client.post("/api/smart-masks/x", json={"settings": {"generator": "nope"}}).status_code == 400
    assert client.post("/api/smart-masks/x", json={"settings": {"generator": "cavity", "generator_level": "high"}}).status_code == 400


@pytest.mark.parametrize("path", ["/api/images/..%2Fprefs.json", "/api/meshes/%2e%2e%2Fprefs.json", "/api/scripts/..%2Fprefs.json", "/api/images/missing.png"])
def test_workspace_file_routes_do_not_leave_their_folder(client, data_dir, path):
    (data_dir / "prefs.json").write_text("{}")
    assert client.get(path).status_code in (404, 400)


def test_workspace_image_is_served_from_its_folder(client, data_dir):
    (data_dir / "images").mkdir(exist_ok=True)
    (data_dir / "images" / "t.png").write_bytes(PNG)
    assert client.get("/api/images/t.png").content == PNG


def test_agent_command_without_a_linked_editor_is_409_and_bad_bodies_are_400(client):
    assert client.post("/api/agent", json={"command": "status"}).status_code == 409
    assert client.post("/api/agent", json={}).status_code == 400
    assert client.post("/api/agent", json={"command": "status", "args": [1]}).status_code == 400


def test_command_bus_runs_known_commands_and_refuses_unknown_ones(client):
    ids = [c["id"] for c in client.get("/api/commands").json()["commands"]]
    assert "prefs.get" in ids
    assert client.post("/api/commands/prefs.get", json={}).json()["ok"] is True
    assert client.post("/api/commands/nothing.here", json={}).status_code == 404
    assert client.post("/api/commands/prefs.set", json={"values": {"nonsense": 1}}).status_code == 400


def test_preferences_are_saved_in_the_data_folder_only(client, data_dir):
    r = client.post("/api/commands/prefs.set", json={"values": {"externalEditor": "gimp"}})
    assert r.status_code == 200
    assert json.loads((data_dir / "prefs.json").read_text())["externalEditor"] == "gimp"
    assert client.post("/api/commands/prefs.get", json={}).json()["result"]["externalEditor"] == "gimp"


def test_command_bus_refuses_a_foreign_origin_even_when_the_guard_is_bypassed(app):
    from fastapi.testclient import TestClient
    from app.agent import router

    assert router._local("http://evil.example") is False
    assert router._local("http://localhost:5176") is True
    assert router._local("tauri://localhost") is True
    assert router._local(None) is True
