import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { OBJLoader } from 'three/addons/loaders/OBJLoader.js';
import { FBXLoader } from 'three/addons/loaders/FBXLoader.js';

export interface FlatMesh {
  positions: Float32Array;
  normals: Float32Array;
  uvs: Float32Array;
  indices: Uint32Array;
  skipped: number;
}

function flatten(root: THREE.Object3D): FlatMesh {
  root.updateMatrixWorld(true);
  const positions: number[] = [];
  const normals: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  let skipped = 0;
  const p = new THREE.Vector3();
  const n = new THREE.Vector3();
  const normalMatrix = new THREE.Matrix3();
  root.traverse((object) => {
    const mesh = object as THREE.Mesh;
    if (!mesh.isMesh || !mesh.geometry) return;
    const geometry = mesh.geometry as THREE.BufferGeometry;
    const pos = geometry.getAttribute('position');
    const uv = geometry.getAttribute('uv');
    if (!pos || !uv) { skipped++; return; }
    if (!geometry.getAttribute('normal')) geometry.computeVertexNormals();
    const nor = geometry.getAttribute('normal');
    normalMatrix.getNormalMatrix(mesh.matrixWorld);
    const base = positions.length / 3;
    for (let i = 0; i < pos.count; i++) {
      p.fromBufferAttribute(pos, i).applyMatrix4(mesh.matrixWorld);
      n.fromBufferAttribute(nor, i).applyMatrix3(normalMatrix).normalize();
      positions.push(p.x, p.y, p.z);
      normals.push(n.x, n.y, n.z);
      uvs.push(uv.getX(i), 1 - uv.getY(i));
    }
    const index = geometry.getIndex();
    if (index) for (let i = 0; i < index.count; i++) indices.push(base + index.getX(i));
    else for (let i = 0; i < pos.count; i++) indices.push(base + i);
  });
  return { positions: new Float32Array(positions), normals: new Float32Array(normals), uvs: new Float32Array(uvs), indices: new Uint32Array(indices), skipped };
}

export async function loadMeshFile(file: File): Promise<FlatMesh> {
  const name = file.name.toLowerCase();
  const data = await file.arrayBuffer();
  if (name.endsWith('.glb') || name.endsWith('.gltf')) {
    const gltf = await new GLTFLoader().parseAsync(data, '');
    return flatten(gltf.scene);
  }
  if (name.endsWith('.obj')) return flatten(new OBJLoader().parse(new TextDecoder().decode(data)));
  if (name.endsWith('.fbx')) return flatten(new FBXLoader().parse(data, ''));
  throw new Error('Unsupported mesh format. Use glTF, GLB, OBJ or FBX.');
}

export function sampleMesh(kind: string): FlatMesh {
  const geometry =
    kind === 'knot' ? new THREE.TorusKnotGeometry(0.7, 0.24, 360, 48) :
    kind === 'torus' ? new THREE.TorusGeometry(0.8, 0.32, 96, 192) :
    kind === 'capsule' ? new THREE.CapsuleGeometry(0.5, 1.0, 16, 48) :
    new THREE.SphereGeometry(1, 128, 64);
  return flatten(new THREE.Mesh(geometry));
}
