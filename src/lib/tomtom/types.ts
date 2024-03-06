import { Point } from '../..';
import { RouteStats } from '../types';

export interface TomTomRouteResponse {
	routes: TomTomRoute[];
}

export interface TomTomRoute {
	summary: TomTomSummary;
	legs: TomTomLeg[];
	sections: TomTomSection[];
}

export interface TomTomSummary {
	lengthInMeters: number;
	travelTimeInSeconds: number;
	trafficDelayInSeconds: number;
	trafficLengthInMeters: number;
	departureTime: string;
	arrivalTime: string;
	noTrafficTravelTimeInSeconds: number;
	historicTrafficTravelTimeInSeconds: number;
	liveTrafficIncidentsTravelTimeInSeconds: number;
}

export interface TomTomLeg {
	summary: TomTomSummary;
	points: Point[];
}

export interface TomTomSection {
	startPointIndex: number;
	endPointIndex: number;
	sectionType: string;
	travelMode: 'car';
}

export interface TomTomSectionTraffic {
	startPointIndex: number;
	endPointIndex: number;
	sectionType: 'TRAFFIC';
	simpleCategory: string;
	effectiveSpeedInKmh: number;
	delayInSeconds: number;
	magnitudeOfDelay: number;
	tec: TomTomTec;
}

export interface TomTomTec {
	causes: TomTomCause[];
	effectCode: number;
}

export interface TomTomCause {
	mainCauseCode: number;
}

export function tomtomRouteToPoints(route: TomTomRouteResponse): Point[] {
	return route.routes[0].legs.flatMap((l) => l.points);
}

export function tomtomRouteStats(route: TomTomRouteResponse): RouteStats {
	const tomtomLength = route.routes[0].summary.lengthInMeters;
	const tomtomDuration = route.routes[0].summary.noTrafficTravelTimeInSeconds;
	const tomtomDurationTraffic =
		route.routes[0].summary.liveTrafficIncidentsTravelTimeInSeconds;

	return {
		distance: tomtomLength,
		duration: tomtomDuration,
		durationTraffic: tomtomDurationTraffic,
		points: tomtomRouteToPoints(route),
	};
}

export function isTomTomRouteResponse(
	response: any
): response is TomTomRouteResponse {
	return (
		Array.isArray(response.routes) &&
		response.routes.length > 0 &&
		'legs' in response.routes[0]
	);
}
