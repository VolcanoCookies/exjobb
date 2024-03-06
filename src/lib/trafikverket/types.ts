import { Point } from '../../index.js';

export interface TrafikVerketResponseRaw<T> {
	RESPONSE: {
		RESULT: T[];
	};
}

export interface TrafikVerketTrafficFlowResponse {
	TrafficFlow: TrafikVerketTrafficFlow[];
	LastChangeId: number | undefined;
}

export interface TrafikVerketTrafficFlow {
	SiteId: number;
	MeasurementTime: Date;
	MeasurementOrCalculationPeriod: number;
	VehicleType: string;
	VehicleFlowRate: number;
	AverageVehicleSpeed: number;
	Geometry: {
		SWEREF99TM: string;
		WGS84: string;
		Point: Point;
	};
	ModifiedTime: Date;
	SpecificLane: string;
	MeasurementSide: string;
}
