import { configDotenv } from 'dotenv';
import { sleep } from '../lib/utils.js';
import { connectDb } from '../db.js';
import { BingClient } from '../lib/bing/client.js';
import { TomTomClient } from '../lib/tomtom/client.js';
import { HereClient } from '../lib/here/client.js';
import { Point } from '../index.js';
import { get_bearing } from '../utils.js';
import { saveResponse as saveRouteResponse } from '../model/routeModel.js';

configDotenv();

async function main() {
	await connectDb();

	const bingKey = process.env.BING_API_KEY;
	const tomtomKey = process.env.TOMTOM_API_KEY;
	const hereKey = process.env.HERE_API_KEY;

	if (!bingKey) {
		throw new Error('BING_API_KEY not found');
	}
	if (!tomtomKey) {
		throw new Error('TOMTOM_API_KEY not found');
	}
	if (!hereKey) {
		throw new Error('HERE_API_KEY not found');
	}

	const bingClient = new BingClient(bingKey);
	const tomtomClient = new TomTomClient(tomtomKey);
	const hereClient = new HereClient(hereKey);

	const routes: { points: Point[]; heading: number }[] = [
		[
			{ latitude: 59.2963961206535, longitude: 18.06193878000687 },
			{ latitude: 59.296385164428706, longitude: 18.051081197914584 },
		],
		[
			{ latitude: 59.32399956066942, longitude: 17.895681500869905 },
			{ latitude: 59.32412682317982, longitude: 17.896459341484224 },
		],
		[
			{ latitude: 59.31196751124371, longitude: 18.150518351883377 },
			{ latitude: 59.31186895022427, longitude: 18.14998593339392 },
		],
	].map((points) => ({
		points,
		heading: get_bearing(points[0], points[1]),
	}));

	let bingFrequency = 1 * 60 * 1000;
	let tomtomFrequency = 5 * 60 * 1000;
	let hereFrequency = 5 * 60 * 1000;

	let duration = 24 * 60 * 60 * 1000;

	exitDelay(duration);

	setInterval(async () => {
		for (const route of routes) {
			try {
				const res = await bingClient.getRoute(
					route.points,
					route.heading
				);
				await saveRouteResponse(undefined, route.points, res);
			} catch (error) {
				console.error(error);
			}
		}
	}, bingFrequency);
	/*
	setInterval(async () => {
		for (const route of routes) {
			try {
				const res = await tomtomClient.getRoute(
					route.points,
					route.heading
				);
				await saveRouteResponse(undefined, route.points, res);
			} catch (error) {
				console.error(error);
			}
		}
	}, tomtomFrequency);
	setInterval(async () => {
		for (const route of routes) {
			try {
				const res = await hereClient.getRoute(
					route.points[0],
					route.points[1],
					undefined,
					route.heading
				);
				await saveRouteResponse(undefined, route.points, res);
			} catch (error) {
				console.error(error);
			}
		}
	}, hereFrequency);
	*/
}

async function exitDelay(delay: number) {
	await sleep(delay);
	process.exit(0);
}

export default main();
