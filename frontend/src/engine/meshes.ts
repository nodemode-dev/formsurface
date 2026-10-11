import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { OBJLoader } from 'three/addons/loaders/OBJLoader.js';
import { FBXLoader } from 'three/addons/loaders/FBXLoader.js';
import { USDLoader } from 'three/addons/loaders/USDLoader.js';
import { PLYLoader } from 'three/addons/loaders/PLYLoader.js';
import { ColladaLoader } from 'three/addons/loaders/ColladaLoader.js';

export interface FlatMesh {
  positions: Float32Array;
  normals: Float32Array;
  uvs: Float32Array;
  indices: Uint32Array;
  parts: Uint32Array;
  materials: Uint32Array;
  materialNames: string[];
  partNames: string[];
  skipped: number;
}

function flatten(root: THREE.Object3D, requireUv = true): FlatMesh {
  root.updateMatrixWorld(true);
  const positions: number[] = [];
  const normals: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  const parts: number[] = [];
  const materials: number[] = [];
  const materialNames: string[] = [];
  const partNames: string[] = [];
  const materialId = (material: THREE.Material | undefined, fallback: string): number => {
    const name = (material?.name || fallback).trim();
    let id = materialNames.indexOf(name);
    if (id < 0) { id = materialNames.length; materialNames.push(name); }
    return id;
  };
  let part = 0;
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
    if (!pos || (!uv && requireUv)) { skipped++; return; }
    if (!geometry.getAttribute('normal')) geometry.computeVertexNormals();
    const nor = geometry.getAttribute('normal');
    normalMatrix.getNormalMatrix(mesh.matrixWorld);
    const base = positions.length / 3;
    for (let i = 0; i < pos.count; i++) {
      p.fromBufferAttribute(pos, i).applyMatrix4(mesh.matrixWorld);
      n.fromBufferAttribute(nor, i).applyMatrix3(normalMatrix).normalize();
      positions.push(p.x, p.y, p.z);
      normals.push(n.x, n.y, n.z);
      if (uv) uvs.push(uv.getX(i), 1 - uv.getY(i)); else uvs.push(0, 0);
    }
    const before = indices.length;
    const index = geometry.getIndex();
    if (index) for (let i = 0; i < index.count; i++) indices.push(base + index.getX(i));
    else for (let i = 0; i < pos.count; i++) indices.push(base + i);
    const count = (indices.length - before) / 3;
    const list = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    const perTriangle = new Array<number>(count).fill(materialId(list[0], 'default'));
    if (Array.isArray(mesh.material)) {
      for (const group of geometry.groups) {
        const slot = group.materialIndex ?? 0;
        const id = materialId(list[slot], `material_${slot}`);
        for (let t = Math.floor(group.start / 3); t < Math.min(count, Math.floor((group.start + group.count) / 3)); t++) perTriangle[t] = id;
      }
    }
    for (let t = 0; t < count; t++) { parts.push(part); materials.push(perTriangle[t]); }
    partNames.push(mesh.name || mesh.parent?.name || '');
    part++;
  });
  return {
    positions: new Float32Array(positions), normals: new Float32Array(normals), uvs: new Float32Array(uvs), indices: new Uint32Array(indices),
    parts: new Uint32Array(parts), materials: new Uint32Array(materials), materialNames, partNames, skipped,
  };
}

export const MESH_EXTENSIONS = ['.glb', '.gltf', '.obj', '.fbx', '.usd', '.usda', '.usdc', '.usdz', '.ply', '.dae'];

async function parseScene(name: string, data: ArrayBuffer): Promise<THREE.Object3D> {
  const lower = name.toLowerCase();
  if (lower.endsWith('.glb') || lower.endsWith('.gltf')) return (await new GLTFLoader().parseAsync(data, '')).scene;
  if (lower.endsWith('.obj')) return new OBJLoader().parse(new TextDecoder().decode(data));
  if (lower.endsWith('.fbx')) return new FBXLoader().parse(data, '');
  if (/\.usd[acz]?$/.test(lower)) {
    return await new Promise<THREE.Object3D>((resolve, reject) => {
      try { new USDLoader().parse(data, '', resolve, reject); } catch (error) { reject(error); }
    });
  }
  if (lower.endsWith('.ply')) {
    const geometry = new PLYLoader().parse(data);
    const mesh = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial());
    mesh.name = name.replace(/\.[^.]+$/, '');
    return mesh;
  }
  if (lower.endsWith('.dae')) {
    const collada = new ColladaLoader().parse(new TextDecoder().decode(data), '');
    if (!collada) throw new Error(`${name} is not a readable Collada file`);
    return collada.scene;
  }
  throw new Error(`Unsupported mesh format. Use ${MESH_EXTENSIONS.join(', ')}.`);
}

export async function loadHighPoly(name: string, data: ArrayBuffer): Promise<FlatMesh> {
  return flatten(await parseScene(name, data), false);
}

function bumpySphere(radius: number, x: number, name: string, segments = 192): THREE.Mesh {
  const geometry = new THREE.SphereGeometry(radius, segments * 2, segments);
  const pos = geometry.getAttribute('position');
  const v = new THREE.Vector3();
  for (let i = 0; i < pos.count; i++) {
    v.fromBufferAttribute(pos, i);
    v.multiplyScalar(1 + 0.03 * Math.sin(v.x * 30) * Math.sin(v.y * 30) * Math.sin(v.z * 30));
    pos.setXYZ(i, v.x, v.y, v.z);
  }
  geometry.computeVertexNormals();
  const mesh = new THREE.Mesh(geometry);
  mesh.position.x = x;
  mesh.name = name;
  return mesh;
}

function samplePair(): FlatMesh {
  const group = new THREE.Group();
  for (const [x, name] of [[-0.55, 'left_low'], [0.55, 'right_low']] as const) {
    const mesh = namedMesh(new THREE.SphereGeometry(0.5, 64, 32), 'pair');
    mesh.position.x = x;
    mesh.name = name;
    if (x > 0) {
      const uv = mesh.geometry.getAttribute('uv');
      for (let i = 0; i < uv.count; i++) uv.setX(i, uv.getX(i) * 0.5 + 0.5);
    } else {
      const uv = mesh.geometry.getAttribute('uv');
      for (let i = 0; i < uv.count; i++) uv.setX(i, uv.getX(i) * 0.5);
    }
    group.add(mesh);
  }
  return flatten(group);
}

export function sampleHighPoly(kind = 'sphere'): FlatMesh {
  if (kind === 'pair') {
    const group = new THREE.Group();
    group.add(bumpySphere(0.5, -0.55, 'left_high'), bumpySphere(0.5, 0.55, 'right_high'));
    return flatten(group, false);
  }
  const group = new THREE.Group();
  for (const half of [0, 1]) {
    const geometry = new THREE.SphereGeometry(1, 384, 192, 0, Math.PI * 2, half * Math.PI / 2, Math.PI / 2);
    const pos = geometry.getAttribute('position');
    const v = new THREE.Vector3();
    for (let i = 0; i < pos.count; i++) {
      v.fromBufferAttribute(pos, i);
      const bumps = 0.025 * Math.sin(v.x * 14) * Math.sin(v.y * 14) * Math.sin(v.z * 14) + (Math.abs(v.y) < 0.05 ? -0.02 : 0);
      v.multiplyScalar(1 + bumps);
      pos.setXYZ(i, v.x, v.y, v.z);
    }
    geometry.computeVertexNormals();
    group.add(new THREE.Mesh(geometry));
  }
  return flatten(group, false);
}

export async function loadMeshFile(file: File): Promise<FlatMesh> {
  return flatten(await parseScene(file.name, await file.arrayBuffer()));
}

function namedMesh(geometry: THREE.BufferGeometry, material: string): THREE.Mesh {
  const standard = new THREE.MeshStandardMaterial();
  standard.name = material;
  return new THREE.Mesh(geometry, standard);
}

function sampleSets(): FlatMesh {
  const group = new THREE.Group();
  group.add(namedMesh(new THREE.CapsuleGeometry(0.5, 1.0, 16, 48), 'body'));
  const ring = namedMesh(new THREE.TorusGeometry(0.56, 0.09, 32, 128), 'trim');
  ring.rotation.x = Math.PI / 2;
  group.add(ring);
  const headGeometry = new THREE.SphereGeometry(0.34, 64, 32);
  const uv = headGeometry.getAttribute('uv');
  for (let i = 0; i < uv.count; i++) uv.setX(i, uv.getX(i) + 1);
  const head = namedMesh(headGeometry, 'body');
  head.position.y = 1.32;
  group.add(head);
  return flatten(group);
}

export function sampleMesh(kind: string): FlatMesh {
  if (kind === 'sets') return sampleSets();
  if (kind === 'pair') return samplePair();
  const geometry =
    kind === 'knot' ? new THREE.TorusKnotGeometry(0.7, 0.24, 360, 48) :
    kind === 'torus' ? new THREE.TorusGeometry(0.8, 0.32, 96, 192) :
    kind === 'capsule' ? new THREE.CapsuleGeometry(0.5, 1.0, 16, 48) :
    new THREE.SphereGeometry(1, 128, 64);
  return flatten(new THREE.Mesh(geometry));
}
