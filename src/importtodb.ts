import { configDotenv } from 'dotenv';
import { readFileSync, readdirSync } from 'fs';
import mongoose from 'mongoose';
import { TrafikverketFlowEntryModel } from './model/trafikverketFlowModel.js';
import { TrafikVerketTrafficFlowResponse } from './lib/trafikverket/types.js';

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

	const dir = './data/flows/2024-02-25';
	const files = readdirSync(dir);
	for (const file of files) {
		console.log(`Processing ${file}`);
		const raw = readFileSync(`${dir}/${file}`, 'utf-8');
		const data = JSON.parse(raw) as TrafikVerketTrafficFlowResponse;

		console.log(`Saving ${data.TrafficFlow.length} entries`);

		const inserts = data.TrafficFlow.map(async (flow) => {
			return TrafikverketFlowEntryModel.create({
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

		await Promise.all(inserts);
	}
}

export default main();
