import { Point } from '../..';
import { RouteStats } from '../types';

export interface BingRouteResponse {
	resourceSets: ResourceSet[];
}

export interface ResourceSet {
	estimatedTotal: number;
	resources: Resource[];
}

export type TrafficCongestion =
	| 'Unknown'
	| 'None'
	| 'Mild'
	| 'Medium'
	| 'Heavy';
export type TrafficDataUsed = 'None' | 'Flow' | 'Closure' | 'FlowAndClosure';
export type TravelMode = 'Driving' | 'Walking' | 'Transit';

export interface Resource {
	bbox: number[];
	id: string;
	distanceUnit: string;
	durationUnit: string;
	routeLegs: RouteLeg[];
	trafficCongestion: TrafficCongestion;
	trafficDataUsed: TrafficDataUsed;
	travelDistance: number;
	travelDuration: number;
	travelDurationTraffic: number;
	travelMode: TravelMode;
	routePath: RoutePath | undefined;
}

export interface RoutePath {
	line: Line;
}

export interface Line {
	coordinates: number[][];
}

export interface RouteLeg {
	actualStart: BingPoint;
	actualEnd: BingPoint;
	itineraryItems: ItineraryItem[];
	travelDistance: number;
	travelDuration: number;
}

export interface ItineraryItem {
	compassDirection: string;
	maneuverPoint: BingPoint;
}

export type Location = [number, number];

export interface BingPoint {
	type: string;
	coordinates: Location;
}

export function bingRouteToPoints(route: BingRouteResponse): Point[] {
	const resources = route.resourceSets[0].resources[0];

	if (resources.routePath === undefined) {
		return resources.routeLegs
			.flatMap((l) => [
				l.actualStart,
				...l.itineraryItems.map((i) => i.maneuverPoint),
				l.actualEnd,
			])
			.map((p) => {
				return {
					latitude: p.coordinates[0],
					longitude: p.coordinates[1],
				};
			});
	} else {
		return resources.routePath!.line.coordinates.map((c) => {
			return { latitude: c[0], longitude: c[1] };
		});
	}
}

export function bingRouteStats(route: BingRouteResponse): RouteStats {
	const bingLength = route.resourceSets[0].resources[0].travelDistance * 1000;
	const bingDuration = route.resourceSets[0].resources[0].travelDuration;
	const bingDurationTraffic =
		route.resourceSets[0].resources[0].travelDurationTraffic;

	return {
		distance: bingLength,
		duration: bingDuration,
		durationTraffic: bingDurationTraffic,
		points: bingRouteToPoints(route),
	};
}

export function isBingRouteResponse(
	response: any
): response is BingRouteResponse {
	return 'resourceSets' in response;
}
