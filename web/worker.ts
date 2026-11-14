// Runs the Equi-X solver off the main thread (see solve.ts).
import wasmUrl from './solver.wasm?url';

type Solver = { memory: WebAssembly.Memory; buffer(): number; solve(effort: number): void };

const SEED = 32;
const NONCE = 16;
const SOLUTION = 16;

const hex = (bytes: Uint8Array) =>
	Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');

self.onmessage = async ({ data }: MessageEvent<{ seed: string; effort: number }>) => {
	const bytes = await (await fetch(wasmUrl)).arrayBuffer();
	const { instance } = await WebAssembly.instantiate(bytes);
	const { memory, buffer, solve } = instance.exports as unknown as Solver;

	// In: seed ‖ start nonce (random, so retries don't redo the same work).
	const io = new Uint8Array(memory.buffer, buffer(), SEED + NONCE + SOLUTION);
	io.set(Uint8Array.from(data.seed.match(/../g) ?? [], (h) => parseInt(h, 16)));
	crypto.getRandomValues(io.subarray(SEED, SEED + NONCE));
	solve(data.effort);

	// Out: nonce ‖ solution, written back at the start of the buffer.
	const out = new Uint8Array(memory.buffer, buffer(), NONCE + SOLUTION);
	postMessage({ nonce: hex(out.subarray(0, NONCE)), solution: hex(out.subarray(NONCE)) });
};
