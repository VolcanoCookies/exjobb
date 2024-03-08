import { configDotenv } from 'dotenv';
import { TrafikVerketClient } from '../lib/trafikverket/client.js';
import { readFile, readFileSync, readdirSync, writeFileSync } from 'fs';
import { save_flow } from '../utils.js';
import { sleep } from '../lib/utils.js';
import mongoose from 'mongoose';
import { TrafikverketFlowEntryModel } from '../model/trafikverketFlowModel.js';
import { Logger } from '@pieropatron/tinylogger';
import { AxiosError } from 'axios';
import { connectDb } from '../db.js';

configDotenv();
const logger = new Logger('trafikverketflow');

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
				latitude: 59.325484,
				longitude: 18.0653,
				radius: 100 * 1000,
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

		res.TrafficFlow.forEach((flow) => {
			TrafikverketFlowEntryModel.create({
				SiteId: flow.SiteId,
				MeasurementTime: flow.MeasurementTime,
				MeasurementOrCalculationPeriod:
					flow.MeasurementOrCalculationPeriod,
				VehicleType: flow.VehicleType,
				VehicleFlowRate: flow.VehicleFlowRate,
				AverageVehicleSpeed: flow.AverageVehicleSpeed,
				Point: {
					latitude: flow.Geometry.Point.latitude,
					longitude: flow.Geometry.Point.longitude,
				},
				ModifiedTime: flow.ModifiedTime,
				SpecificLane: flow.SpecificLane,
				MeasurementSide: flow.MeasurementSide,
			});
		});

		logger.info('Saved', res.TrafficFlow.length, 'entries');
	}, 60 * 1000);
}

export default main();
