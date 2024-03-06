import axios, { AxiosInstance } from 'axios';
import { BingRouteResponse, Location as BingLocation } from './types';
import { Point } from '../..';

function postProcess(response: BingRouteResponse) {}

export class BingClient {
	private apiKey: string;
	private client: AxiosInstance;

	constructor(apiKey: string) {
		this.apiKey = apiKey;
		this.client = axios.create();
	}

	async getRoute(
		waypoints: Point[],
		heading: number
	): Promise<BingRouteResponse> {
		if (waypoints.length < 2) {
			throw new Error('At least two waypoints are required');
		} else if (waypoints.length > 25) {
			throw new Error('Maximum 25 waypoints are allowed');
		}

		let wp = '';
		for (let i = 0; i < waypoints.length; i++) {
			wp += `&wp.${i + 1}=${waypoints[i].latitude},${
				waypoints[i].longitude
			}`;
		}

		const response = await this.client.get(
			`http://dev.virtualearth.net/REST/v1/Routes?${wp}&heading=${heading.toFixed(
				0
			)}&distanceUnit=km&key=${
				this.apiKey
			}&optimize=timeWithTraffic&routeAttributes=routePath`
		);
		let data = response.data;
		postProcess(data);
		return data;
	}
}
