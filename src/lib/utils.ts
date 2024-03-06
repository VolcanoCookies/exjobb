import { readdirSync, statSync } from 'fs';

export function distanceMeters(a1: number, a2: number, b1: number, b2: number) {
	const R = 6371e3; // metres
	const φ1 = (a1 * Math.PI) / 180; // φ, λ in radians
	const φ2 = (b1 * Math.PI) / 180;
	const Δφ = ((b1 - a1) * Math.PI) / 180;
	const Δλ = ((b2 - a2) * Math.PI) / 180;

	const a =
		Math.sin(Δφ / 2) * Math.sin(Δφ / 2) +
		Math.cos(φ1) * Math.cos(φ2) * Math.sin(Δλ / 2) * Math.sin(Δλ / 2);
	const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));

	const d = R * c;
	return d; // in metres
}

export function validateRange(value: number, min: number, max: number) {
	if (value < min || value > max) {
		throw new Error(`Value ${value} not in range ${min}-${max}`);
	}
}

export function sleep(ms: number) {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

export function readDirRecursiveSync(path: string): string[] {
	const files = readdirSync(path);
	return files.flatMap((f) => {
		const fullPath = `${path}/${f}`;
		const stats = statSync(fullPath);
		if (stats.isDirectory()) {
			return readDirRecursiveSync(fullPath);
		} else {
			return fullPath;
		}
	});
}
