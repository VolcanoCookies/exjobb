import { configDotenv } from 'dotenv';
import { TrafikVerketClient } from '../lib/trafikverket/client.js';
import {
	TrafikverketFlowEntryModel,
	TrafikverketSiteEntryModel,
} from '../model/trafikverketFlowModel.js';
import { Logger } from '@pieropatron/tinylogger';
import { AxiosError } from 'axios';
import { connectDb } from '../db.js';
import { Point } from '../index.js';

configDotenv();
const logger = new Logger('trafikverketflow');

/// Configuration
// Center of a circle, we want to filter out sensors outside of this circle
const center = {
	latitude: 59.325484,
	longitude: 18.0653,
};
// Radius of the circle, in meters
const radius = 100 * 1000; // 100km
// Frequency of fetching data, in milliseconds
const frequency = 60 * 1000; // 1 minute

async function main() {
	await connectDb();

	const trafikverketKey = process.env.TRAFIKVERKET_API_KEY;
	if (!trafikverketKey) {
		throw new Error('TRAFFICVERKET_API_KEY not found');
	}

	const client = new TrafikVerketClient(trafikverketKey);

	let lastChangeId = 0;
	setInterval(async () => {
		// All traffic around center of stockholm in a 100km radius
		const res = await client
			.getAllTrafficFlow(10000, lastChangeId, {
				...center,
				radius,
			})
			.catch((err) => {
				if (err instanceof AxiosError) {
					logger.warn(
						`Error fetching traffic flow, status: ${err.response?.status}, message: ${err.response?.data}`
					);
				} else {
					logger.error('Error fetching traffic flow, ', err);
				}
			});

		if (!res) {
			logger.info('No new data');
			return;
		}

		lastChangeId = res.LastChangeId!;

		const sites = new Map<number, Point>();
		res.TrafficFlow.forEach((flow) => {
			sites.set(flow.SiteId, {
				latitude: flow.Geometry.Point.latitude,
				longitude: flow.Geometry.Point.longitude,
			});

			TrafikverketFlowEntryModel.create({
				SiteId: flow.SiteId,
				MeasurementTime: flow.MeasurementTime,
				MeasurementOrCalculationPeriod:
					flow.MeasurementOrCalculationPeriod,
				VehicleType: flow.VehicleType,
				VehicleFlowRate: flow.VehicleFlowRate,
				AverageVehicleSpeed: flow.AverageVehicleSpeed,
				location: {
					type: 'Point',
					coordinates: [
						flow.Geometry.Point.longitude,
						flow.Geometry.Point.latitude,
					],
				},
				ModifiedTime: flow.ModifiedTime,
				SpecificLane: flow.SpecificLane,
				MeasurementSide: flow.MeasurementSide,
			});
		});

		sites.forEach((point, siteId) => {
			TrafikverketSiteEntryModel.updateOne(
				{ SiteId: siteId },
				{
					$setOnInsert: {
						SiteId: siteId,
						location: {
							type: 'Point',
							coordinates: [point.longitude, point.latitude],
						},
					},
				},
				{ upsert: true }
			);
		});

		logger.info('Saved', res.TrafficFlow.length, 'entries');
	}, frequency);
}

export default main();
