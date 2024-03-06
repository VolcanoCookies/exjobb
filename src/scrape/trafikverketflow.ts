import { configDotenv } from 'dotenv';
import { TrafikVerketClient } from '../lib/trafikverket/client.js';
import { readFile, readFileSync, readdirSync, writeFileSync } from 'fs';
import { save_flow } from '../utils.js';
import { sleep } from '../lib/utils.js';
import mongoose from 'mongoose';
import { TrafikverketFlowEntryModel } from '../model/trafikverketFlowModel.js';

configDotenv();

async function main() {
	const mongoUrl = process.env.MONGO_URL;
	if (!mongoUrl) {
		throw new Error('MONGO_URL not found');
	}

	await mongoose.connect(mongoUrl, {
		dbName: 'exjobb',
	});

	console.log('Connected to MongoDB');

	const trafikverketKey = process.env.TRAFIKVERKET_API_KEY;
	if (!trafikverketKey) {
		throw new Error('TRAFFICVERKET_API_KEY not found');
	}

	const client = new TrafikVerketClient(trafikverketKey);

	let lastChangeId = 0;
	setInterval(async () => {
		// All traffic around center of stockholm in a 100km radius
		const res = await client.getAllTrafficFlow(10000, lastChangeId, {
			latitude: 59.325484,
			longitude: 18.0653,
			radius: 100 * 1000,
		});

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

		console.log('Saved', res.TrafficFlow.length, 'entries');
	}, 60 * 1000);
}

export default main();
