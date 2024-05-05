import { configDotenv } from 'dotenv';
import { sleep } from '../lib/utils.js';
import { connectDb } from '../db.js';
import { BingClient } from '../lib/bing/client.js';
import { TomTomClient } from '../lib/tomtom/client.js';
import { HereClient } from '../lib/here/client.js';
import { Point } from '../index.js';
import { get_bearing } from '../utils.js';
import { saveResponse as saveRouteResponse } from '../model/routeModel.js';
import { Logger } from '@pieropatron/tinylogger';
import { AxiosError } from 'axios';

configDotenv();
const logger = new Logger('scrape-routes');

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

	const start = {
		latitude: 59.305007,
		longitude: 18.017391,
	};

	const end1km = {
		latitude: 59.31137,
		longitude: 18.006439,
	};
	const end2km = {
		latitude: 59.319604,
		longitude: 17.997351,
	};
	const end4km = {
		latitude: 59.334975,
		longitude: 18.010087,
	};
	const end8km = {
		latitude: 59.356922,
		longitude: 18.032265,
	};

	const routes: { points: Point[]; heading: number }[] = [
		[start, end1km],
		[start, end2km],
		[start, end4km],
		[start, end8km],
	].map((points) => ({
		points,
		heading: get_bearing(points[0], points[1]),
	}));

	const desiredStartDate = Date.parse('2024-05-06T05:00:00+02:00');
	const now = Date.now();
	const delay = desiredStartDate - now;
	if (delay < 0) {
		logger.error('Desired start date is in the past');
		process.exit(1);
	} else {
		let seconds = Math.floor(delay / 1000);
		let minutes = Math.floor(seconds / 60);
		let hours = Math.floor(minutes / 60);
		seconds = seconds % 60;
		minutes = minutes % 60;
		logger.info(
			`Waiting ${hours}:${minutes}:${seconds} until desired start date`
		);
		await sleep(delay);
	}

	const frequency = 60 * 1000;
	let duration = 8 * 3600 * 1000;

	exitDelay(duration);

	const bingLogger = new Logger('bing');
	const tomtomLogger = new Logger('tomtom');
	const hereLogger = new Logger('here');

	interface Route {
		points: Point[];
		heading: number;
	}

	async function doBingRequest(route: Route) {
		try {
			return await bingClient.getRoute(route.points, route.heading);
		} catch (error) {
			if (error instanceof AxiosError) {
				bingLogger.warn(
					`HTTP error while scraping bing, status: ${error.response?.status}, message: ${error.response?.data}`
				);
			} else {
				bingLogger.warn('Request error while scraping bing');
				bingLogger.error(error);
			}
		}
	}

	async function doTomTomRequest(route: Route) {
		try {
			return await tomtomClient.getRoute(route.points, route.heading);
		} catch (error) {
			if (error instanceof AxiosError) {
				tomtomLogger.warn(
					`HTTP error while scraping tomtom, status: ${error.response?.status}, message: ${error.response?.data}`
				);
			} else {
				tomtomLogger.warn('Request error while scraping tomtom');
				tomtomLogger.error(error);
			}
		}
	}

	async function doHereRequest(route: Route) {
		try {
			return await hereClient.getRoute(
				route.points[0],
				route.points[1],
				undefined,
				route.heading
			);
		} catch (error) {
			if (error instanceof AxiosError) {
				hereLogger.warn(
					`HTTP error while scraping here, status: ${error.response?.status}, message: ${error.response?.data}`
				);
			} else {
				hereLogger.warn('Request error while scraping here');
				hereLogger.error(error);
			}
		}
	}

	const batchId = 'second-batch';

	let i = 0;
	setInterval(async () => {
		logger.info('Scraping routes ' + i++);
		for (const route of routes) {
			const date = new Date();
			const [bingResponse, tomtomResponse, hereResponse] =
				await Promise.all([
					doBingRequest(route),
					doTomTomRequest(route),
					doHereRequest(route),
				]);

			await saveRouteResponse(batchId, route.points, bingResponse!, date);
			await saveRouteResponse(
				batchId,
				route.points,
				tomtomResponse!,
				date
			);
			await saveRouteResponse(batchId, route.points, hereResponse, date);
		}
	}, frequency);
}

async function exitDelay(delay: number) {
	await sleep(delay);
	process.exit(0);
}

export default main();
