import { Schema, model } from "mongoose";

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
trafikverketFlowEntrySchema.index({ SiteId: 1 }, { unique: false });
trafikverketFlowEntrySchema.index({ MeasurementTime: 1 }, { unique: false });

export const TrafikverketFlowEntryModel = model<TrafikverketFlowEntry>(
  "TrafikverketFlowEntry",
  trafikverketFlowEntrySchema,
  "trafikverketflowentries_v2"
);

export interface TrafikverketSiteEntry {
  SiteId: number;
  location: {
    type: "Point";
    coordinates: [number, number];
  };
  RegionId: number;
  MeasurementSide: string;
  SpecificLane: string;
}

export const trafikverketSiteEntrySchema = new Schema<TrafikverketSiteEntry>({
  SiteId: {
    type: Number,
    required: true,
    unique: false,
    index: true,
  },
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
});

trafikverketSiteEntrySchema.index({ location: "2dsphere" }, { unique: false });
trafikverketSiteEntrySchema.index({ SiteId: 1 }, { unique: false });

export const TrafikverketSiteEntryModel = model<TrafikverketSiteEntry>(
  "TrafikverketSiteEntry",
  trafikverketSiteEntrySchema
);

TrafikverketFlowEntryModel.ensureIndexes();
TrafikverketSiteEntryModel.ensureIndexes();
