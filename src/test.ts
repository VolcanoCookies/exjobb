import { writeFileSync } from 'fs';
import { TrafikVerketClient } from './lib/trafikverket/client.js';

async function main() {
	const latitude = 59.32128;
	const longitude = 18.06736;

	const client = new TrafikVerketClient('***REMOVED***');
	const roadData = await client.getRoadGeometry(
		{ latitude, longitude },
		50 * 1000,
		100000
	);

	writeFileSync('roadData.json', JSON.stringify(roadData, null, 2));

	const sensorData = await client.getAllTrafficFlow(10000, undefined, {
		latitude,
		longitude,
		radius: 50 * 1000,
	});

	writeFileSync(
		'sensorData.json',
		JSON.stringify(sensorData.TrafficFlow, null, 2)
	);
}

export default main();
