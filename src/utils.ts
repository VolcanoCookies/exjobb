import { mkdir, writeFile } from 'fs/promises';
import { BingRouteResponse } from './lib/bing/types.js';
import { HereRouteResponse } from './lib/here/types.js';
import { TomTomRouteResponse } from './lib/tomtom/types.js';
import { Point } from './index.js';
import { TrafikVerketTrafficFlowResponse } from './lib/trafikverket/types.js';

function date_as_string(): string {
	const now = new Date();
	const month = (now.getMonth() + 1).toString().padStart(2, '0');
	const day = now.getDate().toString().padStart(2, '0');
	return `${now.getFullYear()}-${month}-${day}`;
}

export async function save_route(
	data: TomTomRouteResponse | BingRouteResponse | HereRouteResponse,
	name: string
): Promise<string> {
	const path = `data/routes/${date_as_string()}/${name}`;

	const dir = path.split('/').slice(0, -1).join('/');
	await mkdir(dir, { recursive: true });

	return writeFile(path, JSON.stringify(data, null, 2)).then(() => path);
}

export async function save_flow(
	data: TrafikVerketTrafficFlowResponse,
	name: string
): Promise<string> {
	const path = `data/flows/${date_as_string()}/${name}`;

	const dir = path.split('/').slice(0, -1).join('/');
	await mkdir(dir, { recursive: true });

	return writeFile(path, JSON.stringify(data, null, 2)).then(() => path);
}

export function get_bearing(a: Point, b: Point): number {
	const lat1 = a.latitude * (Math.PI / 180);
	const long1 = a.longitude * (Math.PI / 180);

	const lat2 = b.latitude * (Math.PI / 180);
	const long2 = b.longitude * (Math.PI / 180);

	let bearing = Math.atan2(
		Math.sin(long2 - long1) * Math.cos(lat2),
		Math.cos(lat1) * Math.sin(lat2) -
			Math.sin(lat1) * Math.cos(lat2) * Math.cos(long2 - long1)
	);

	bearing = bearing * (180 / Math.PI);
	bearing = (bearing + 360) % 360;
	return bearing;
}
