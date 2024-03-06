import { InferSchemaType, Schema, model } from 'mongoose';

export interface TrafikverketFlowEntry {
	SiteId: number;
	MeasurementTime: Date;
	MeasurementOrCalculationPeriod: number;
	VehicleType: string;
	VehicleFlowRate: number;
	AverageVehicleSpeed: number;
	Point: {
		latitude: number;
		longitude: number;
	};
	ModifiedTime: Date;
	RegionId: number;
	SpecificLane: string;
	MeasurementSide: string;
}

export const trafikverketFlowEntrySchema = new Schema<TrafikverketFlowEntry>({
	SiteId: {
		type: Number,
		required: true,
		unique: false,
		index: true,
	},
	MeasurementTime: Date,
	MeasurementOrCalculationPeriod: Number,
	VehicleType: String,
	VehicleFlowRate: Number,
	AverageVehicleSpeed: Number,
	Point: {
		latitude: Number,
		longitude: Number,
	},
	ModifiedTime: Date,
	RegionId: Number,
	SpecificLane: String,
	MeasurementSide: String,
});

export const TrafikverketFlowEntryModel = model<TrafikverketFlowEntry>(
	'TrafikverketFlowEntry',
	trafikverketFlowEntrySchema
);
