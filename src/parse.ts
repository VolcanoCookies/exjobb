import { readFileSync } from 'fs';

export async function main() {
	let fileContent = readFileSync('./out.txt', 'utf8');
	let data = JSON.parse(fileContent);
	parse_result(data);
}

function parse_result(data: any) {
	let results = data.results;

	for (let result of results) {
		let location = result['location'];
		if (location['description'] !== 'Huvudstaleden') continue;

		let flow = result['currentFlow'];
		let speed = parseFloat(flow['speed']) * 3.6;
		let speedUncapped = parseFloat(flow['speedUncapped']) * 3.6;
		let freeFlow = parseFloat(flow['freeFlow']) * 3.6;
		let confidence = flow['confidence'];

		console.log(`Speed: ${speed} km/h`);
		console.log(`Speed Uncapped: ${speedUncapped} km/h`);
		console.log(`Free Flow: ${freeFlow} km/h`);
		console.log(`Confidence: ${confidence}`);

		let links = location['shape']['links'];

		let start = links[0]['points'][0];
		start = start['lat'] + ',' + start['lng'];
		let endPoints = links[links.length - 1]['points'];
		let end = endPoints[endPoints.length - 1];
		end = end['lat'] + ',' + end['lng'];

		let length = parseFloat(location['length']);

		console.log(`Length: ${length} m`);
		console.log(`Start: ${start}`);
		console.log(`End: ${end}`);

		console.log('------------------------------------');
	}
}

export default main();
