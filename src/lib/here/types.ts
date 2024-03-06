import { Point } from '../..';
import { decode } from '../polyline.js';
import { RouteStats } from '../types';

export interface HereRouteResponse {
	routes: HereRoute[];
}

export interface HereRoute {
	id: string;
	sections: HereRouteSection[];
}

export interface HereRouteSection {
	id: string;
	type: string;
	departure: HereDeparture;
	arrival: HereArrival;
	summary: HereSummary;
	polyline: string;
	refPlacements: { [key: string]: string };
	transport: HereTransport;
	spans: HereSpan[];
}

export interface HereDeparture {
	time: string;
	place: HerePlace;
	waypoint: number | undefined;
}

export interface HereArrival {
	time: string;
	place: HerePlace;
	waypoint: number | undefined;
}

export interface HerePlace {
	type: string;
	location: HerePoint;
	originalLocation: HerePoint;
}

export interface HereSummary {
	duration: number;
	length: number;
	baseDuration: number;
}

export interface HereTransport {
	mode: string;
}

export interface HereSpan {
	offset: number;
	carAttributes: string[];
	length: number;
	duration: number;
	baseDuration: number;
	typicalDuration: number;
	maxSpeed: number;
	dynamicSpeedInfo: HereDynamicSpeedInfo;
	segmentRef: string;
}

export interface HereDynamicSpeedInfo {
	baseSpeed: number;
	trafficTime: number;
	turnTime: number;
}

export function hereRouteToPoints(route: HereRouteResponse): Point[] {
	return route.routes[0].sections
		.flatMap((s) => {
			return decode(s.polyline).polyline;
		})
		.map((p) => {
			return { latitude: p[0], longitude: p[1] };
		});
}

/* FLOW RESPONSE */

export interface HereFlowResponse {
	sourceUpdated: string;
	results: HereResult[];
}

export interface HereResult {
	location: HereLocation;
	currentFlow: HereFlow;
}

export interface HereLocation {
	description: string | undefined;
	length: number;
	shape: HereShape;
	hash: string;
}

export interface HereShape {
	links: HereLink[];
}

export interface HereLink {
	points: HerePoint[];
	length: number;
}

export interface HerePoint {
	lat: number;
	lng: number;
}

export interface HereFlow {
	speed: number;
	speedUncapped: number;
	freeFlow: number;
	jamFactor: number;
	confidence: number;
	traversability: HereTraversability;
	confidenceIs: HereConfidenceIs;
}

export type HereTraversability = 'open' | 'closed' | 'reversibleNotRoutable';
export type HereConfidenceIs = 'realtime' | 'historical' | 'speedLimit';

export function hereRouteStats(route: HereRouteResponse): RouteStats {
	const hereSections = route.routes[0].sections;
	const hereLength = hereSections
		.map((s) => s.summary.length)
		.reduce((a, b) => a + b);
	const hereDuration = hereSections
		.map((s) => s.summary.baseDuration)
		.reduce((a, b) => a + b);
	const hereDurationTraffic = hereSections
		.map((s) => s.summary.duration)
		.reduce((a, b) => a + b);

	return {
		distance: hereLength,
		duration: hereDuration,
		durationTraffic: hereDurationTraffic,
		points: hereRouteToPoints(route),
	};
}

export function isHereRouteResponse(
	response: any
): response is HereRouteResponse {
	return (
		Array.isArray(response.routes) &&
		response.routes.length > 0 &&
		'id' in response.routes[0]
	);
}
