import { BingRouteResponse } from './lib/bing/types.js';
import { HereRouteResponse } from './lib/here/types.js';
import { TomTomRouteResponse } from './lib/tomtom/types.js';
import { readFileSync } from 'fs';

async function main() {
	const times = 12 * 3;

	const path = 'data/2024-02-09/compareRoutes/';

	for (let i = 0; i < times; i++) {
		const bingRoute: BingRouteResponse = JSON.parse(
			readFileSync(`${path}bing_${i}.json`, 'utf-8')
		);
		const hereRoute: HereRouteResponse = JSON.parse(
			readFileSync(`${path}here_${i}.json`, 'utf-8')
		);
		const tomtomRoute: TomTomRouteResponse = JSON.parse(
			readFileSync(`${path}tomtom_${i}.json`, 'utf-8')
		);

		const bingLength =
			bingRoute.resourceSets[0].resources[0].travelDistance * 1000;
		const bingDuration =
			bingRoute.resourceSets[0].resources[0].travelDuration;
		const bingDurationTraffic =
			bingRoute.resourceSets[0].resources[0].travelDurationTraffic;

		const hereSections = hereRoute.routes[0].sections;
		const hereLength = hereSections
			.map((s) => s.summary.length)
			.reduce((a, b) => a + b);
		const hereDuration = hereSections
			.map((s) => s.summary.baseDuration)
			.reduce((a, b) => a + b);
		const hereDurationTraffic = hereSections
			.map((s) => s.summary.duration)
			.reduce((a, b) => a + b);

		const tomtomLength = tomtomRoute.routes[0].summary.lengthInMeters;
		const tomtomDuration =
			tomtomRoute.routes[0].summary.noTrafficTravelTimeInSeconds;
		const tomtomDurationTraffic =
			tomtomRoute.routes[0].summary
				.liveTrafficIncidentsTravelTimeInSeconds;

		console.log(`==================== ${i} ====================`);
		console.log(
			`Length: Bing: ${bingLength}, Here: ${hereLength}, TomTom: ${tomtomLength}`
		);
		console.log(
			`Duration: Bing: ${bingDuration}, Here: ${hereDuration}, TomTom: ${tomtomDuration}`
		);
		console.log(
			`DurationTraffic: Bing: ${bingDurationTraffic}, Here: ${hereDurationTraffic}, TomTom: ${tomtomDurationTraffic}`
		);
	}
}

export default main();
