import dotenv from 'dotenv';
import {
	BingRouteResponse,
	Location,
	bingRouteStats,
} from './lib/bing/types.js';
import { BingClient } from './lib/bing/client.js';
import { HereRouteResponse, hereRouteStats } from './lib/here/types.js';
import { HereClient } from './lib/here/client.js';
import {
	TomTomRouteResponse,
	TomTomRoute,
	tomtomRouteStats,
} from './lib/tomtom/types.js';
import { TomTomClient } from './lib/tomtom/client.js';
import { Point } from '.';
import { mkdirSync, writeFileSync } from 'fs';
import { save_route } from './utils.js';

dotenv.config();

async function main() {
	const bindApiKey = process.env.BING_API_KEY!;
	const hereApiKey = process.env.HERE_API_KEY!;
	const tomtomApiKey = process.env.TOMTOM_API_KEY!;

	const bingClient = new BingClient(bindApiKey);
	const hereClient = new HereClient(hereApiKey);
	const tomtomClient = new TomTomClient(tomtomApiKey);

	const points: Point[] = [
		{ latitude: 59.27982815056899, longitude: 18.081231887093555 },
		{ latitude: 59.27859915362582, longitude: 18.086302969742313 },
		{ latitude: 59.27711649766581, longitude: 18.086592660111926 },
		{ latitude: 59.276863911991704, longitude: 18.088737212297723 },
		{ latitude: 59.27808850184413, longitude: 18.089304873003 },
		{ latitude: 59.27874476242707, longitude: 18.090307861801858 },
		{ latitude: 59.27914442085388, longitude: 18.089246752061904 },
		{ latitude: 59.279751334000295, longitude: 18.088179400345453 },
		{ latitude: 59.280047295437136, longitude: 18.08738022745227 },
		{ latitude: 59.27999248795774, longitude: 18.086811688347034 },
	];

	const times = 1;

	for (let i = 0; i < times; i++) {
		const bingRoute: BingRouteResponse = await bingClient.getRoute(
			points,
			270
		);
		const hereRoute: HereRouteResponse = await hereClient.getRoute(
			points[0],
			points[points.length - 1],
			points.slice(1, -1),
			270
		);
		const tomtomRoute: TomTomRouteResponse = await tomtomClient.getRoute(
			points,
			270
		);

		save_route(bingRoute, `charlie/bing.json`);
		save_route(hereRoute, `charlie/here.json`);
		save_route(tomtomRoute, `charlie/tomtom.json`);

		const bingStats = bingRouteStats(bingRoute);
		const hereStats = hereRouteStats(hereRoute);
		const tomtomStats = tomtomRouteStats(tomtomRoute);

		console.log(`==================== ${i} ====================`);
		console.log(
			`Length: Bing: ${bingStats.distance}, Here: ${hereStats.distance}, TomTom: ${tomtomStats.distance}`
		);
		console.log(
			`Duration: Bing: ${bingStats.duration}, Here: ${hereStats.duration}, TomTom: ${tomtomStats.duration}`
		);
		console.log(
			`Duration Traffic: Bing: ${bingStats.durationTraffic}, Here: ${hereStats.durationTraffic}, TomTom: ${tomtomStats.durationTraffic}`
		);
	}
}

export default main();
