interface Tree {
  value: number;
  left: Tree | null;
  right: Tree | null;
}

function leaf(value: number): Tree {
  return { value, left: null, right: null };
}

function insert(root: Tree | null, value: number): Tree {
  if (root === null) return leaf(value);
  if (value < root.value) {
    root.left = insert(root.left, value);
  } else {
    root.right = insert(root.right, value);
  }
  return root;
}

function sum(root: Tree | null): number {
  if (!root) return 0;
  return root.value + sum(root.left) + sum(root.right);
}

function depth(root: Tree | null): number {
  if (root === null) {
    return 0;
  }
  return 1 + Math.max(depth(root.left), depth(root.right));
}

function find(root: Tree | null, value: number): Tree | null {
  let current = root;
  while (current !== null && current.value !== value) {
    current = value < current.value ? current.left : current.right;
  }
  return current;
}

let root: Tree | null = null;
for (const v of [5, 3, 8, 1, 4, 9]) {
  root = insert(root, v);
}
console.log(sum(root), depth(root));
const found = find(root, 4);
console.log(found?.value, find(root, 7)?.value ?? -1);
const name: string | undefined = undefined;
console.log(name ?? "anonymous", name?.length ?? 0);
