/**
 * Browser side of the signup/login proof of work. The puzzle is Equi-X (Tor's
 * onion-service PoW); `solver.wasm` is this repository's crate compiled to
 * WebAssembly, rebuilt with `./build-wasm.sh`. It runs in a worker so the page
 * stays responsive, and starts as soon as the page has its challenge: it's
 * usually done before the user submits.
 *
 * Written for a Vite project (`?url` import in worker.ts).
 */

export type PowChallenge = {
	purpose: string;
	seed: string;
	effort: number;
	expires: number;
	signature: string;
};

/** Resolves to the `pow` field the API expects. Aborting stops the worker. */
export function solvePow(challenge: PowChallenge, signal?: AbortSignal): Promise<string> {
	return new Promise((resolve, reject) => {
		const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
		const stop = () => {
			worker.terminate();
			reject(new DOMException('Aborted', 'AbortError'));
		};
		signal?.addEventListener('abort', stop, { once: true });
		worker.onmessage = (e: MessageEvent<{ nonce: string; solution: string }>) => {
			signal?.removeEventListener('abort', stop);
			worker.terminate();
			resolve(btoa(JSON.stringify({ ...challenge, ...e.data })));
		};
		worker.onerror = (e) => {
			signal?.removeEventListener('abort', stop);
			worker.terminate();
			reject(new Error(e.message));
		};
		worker.postMessage({ seed: challenge.seed, effort: challenge.effort });
	});
}
