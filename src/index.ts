import {
	createWriteStream,
	mkdirSync,
	readFileSync,
	readdirSync,
	writeFile,
	writeFileSync,
} from 'fs';

import { HereFlowResponse } from './lib/here/types.js';
import { HereClient } from './lib/here/client.js';
import openlr from 'openlr-js';
import { TrafikVerketClient } from './lib/trafikverket/client.js';
import { sleep } from './lib/utils.js';
import { save_flow } from './utils.js';
import { TrafikVerketTrafficFlowResponse } from './lib/trafikverket/types.js';
const { BinaryDecoder, LocationReference, RawLocationReference, Serializer } =
	openlr;

const BING_API_KEY =
	'***REMOVED***';
const HERE_API_KEY = '***REMOVED***';
const TOMTOM_API_KEY = '***REMOVED***';
const TRAFIKVERKET_API_KEY = '***REMOVED***';

export async function main() {
	const base_path = './data/flows/2024-02-25';
	const files = readdirSync(base_path);

	const siteId = 44631;
	const vehicleTypes = new Set<string>();
	const rows = [];

	for (const file of files) {
		const data = JSON.parse(
			readFileSync(`${base_path}/${file}`, 'utf-8')
		) as TrafikVerketTrafficFlowResponse;

		const out = data.TrafficFlow.filter((flow) => flow.SiteId === siteId);
		const row: {
			[key: string]: {
				count: number;
				speed: number;
				time: Date;
			};
		} = {};
		out.forEach((flow) => {
			row[flow.VehicleType] = {
				count: flow.VehicleFlowRate,
				speed: flow.AverageVehicleSpeed,
				time: flow.MeasurementTime,
			};
			vehicleTypes.add(flow.VehicleType);
		});
		rows.push(row);
	}

	const types = Array.from(vehicleTypes).sort();
	for (const type of types) {
		const stream = createWriteStream(`./data/flows/${type}.csv`);
		stream.write('time,count,speed\n');
		rows.forEach((row) => {
			const data = row[type];
			if (data) {
				stream.write(`${data.time},${data.count},${data.speed}\n`);
			}
		});
		stream.close();
	}
}

export default main();

export interface Point {
	latitude: number;
	longitude: number;
}
