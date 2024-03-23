import { InferSchemaType, Schema, model } from "mongoose";

export interface TrafikverketFlowEntry {
  SiteId: number;
  MeasurementTime: Date;
  MeasurementOrCalculationPeriod: number;
  VehicleType: string;
  VehicleFlowRate: number;
  AverageVehicleSpeed: number;
  location: {
    type: "Point";
    coordinates: [number, number];
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
  location: {
    type: {
      type: String,
      enum: ["Point"],
      required: true,
    },
    coordinates: {
      type: [Number],
      required: true,
    },
  },
  ModifiedTime: Date,
  RegionId: Number,
  SpecificLane: String,
  MeasurementSide: String,
});

trafikverketFlowEntrySchema.index({ location: "2dsphere" }, { unique: false });

export const TrafikverketFlowEntryModel = model<TrafikverketFlowEntry>(
  "TrafikverketFlowEntry",
  trafikverketFlowEntrySchema
);
