import { Point } from '..';

export interface RouteStats {
	distance: number;
	duration: number;
	durationTraffic: number;
	points: Point[];
}
