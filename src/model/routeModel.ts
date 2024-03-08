import { Schema, model } from 'mongoose';
import { Point } from '../index.js';
import { BingRouteResponse, isBingRouteResponse } from '../lib/bing/types.js';
import {
	TomTomRouteResponse,
	isTomTomRouteResponse,
} from '../lib/tomtom/types.js';
import { HereRouteResponse, isHereRouteResponse } from '../lib/here/types.js';

export interface GeoJsonLineString {
	type: 'LineString';
	// Note: [longitude, latitude]
	coordinates: [number, number][];
}

export function pointsToLineString(points: Point[]): GeoJsonLineString {
	return {
		type: 'LineString',
		coordinates: points.map((p) => [p.longitude, p.latitude]),
	};
}

export const GeoJsonLineStringSchema = new Schema<GeoJsonLineString>({
	type: {
		type: String,
		enum: ['LineString'],
		required: true,
	},
	coordinates: {
		type: [[Number]],
		required: true,
	},
});

export interface RouteResponseEntry<T> {
	id: string;
	batchId: string | undefined;
	path: Point[];
	date: Date;
	response: T;
}

export const RouteEntrySchema = new Schema<RouteResponseEntry<any>>({
	batchId: {
		type: String,
		required: false,
		index: true,
	},
	path: GeoJsonLineStringSchema,
	date: {
		type: Date,
		required: true,
	},
	response: {
		type: Object,
		required: true,
	},
});

RouteEntrySchema.index({ path: '2dsphere' }, { unique: false });

export const BingRouteEntryModel = model<RouteResponseEntry<BingRouteResponse>>(
	'BingRouteEntry',
	RouteEntrySchema
);

export const TomTomRouteEntryModel = model<
	RouteResponseEntry<TomTomRouteResponse>
>('TomTomRouteEntry', RouteEntrySchema);

export const HereRouteEntryModel = model<RouteResponseEntry<HereRouteResponse>>(
	'HereRouteEntry',
	RouteEntrySchema
);

export async function saveResponse<T>(
	batchId: string | undefined,
	path: Point[],
	response: BingRouteResponse | TomTomRouteResponse | HereRouteResponse
) {
	if (isBingRouteResponse(response)) {
		await BingRouteEntryModel.create({
			batchId,
			path: pointsToLineString(path),
			date: new Date(),
			response,
		});
	} else if (isTomTomRouteResponse(response)) {
		await TomTomRouteEntryModel.create({
			batchId,
			path: pointsToLineString(path),
			date: new Date(),
			response,
		});
	} else if (isHereRouteResponse(response)) {
		await HereRouteEntryModel.create({
			batchId,
			path: pointsToLineString(path),
			date: new Date(),
			response,
		});
	}
}
