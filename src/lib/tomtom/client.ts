import axios, { AxiosInstance } from 'axios';
import { TomTomRouteResponse } from './types';
import { Point } from '../..';

function postProcess(response: TomTomRouteResponse) {}

export class TomTomClient {
	private apiKey: string;
	private client: AxiosInstance;

	constructor(apiKey: string) {
		this.apiKey = apiKey;
		this.client = axios.create();
	}

	async getRoute(
		points: Point[],
		heading: number
	): Promise<TomTomRouteResponse> {
		const locations = points
			.map((point) => `${point.latitude},${point.longitude}`)
			.join(':');

		const response = await this.client.get(
			`https://api.tomtom.com/routing/1/calculateRoute/${locations}/json`,
			{
				params: {
					routeRepresentation: 'polyline',
					computeTravelTimeFor: 'all',
					vehicleHeading: heading.toFixed(0),
					sectionType: ['traffic'],
					traffic: true,
					key: this.apiKey,
				},
				paramsSerializer: {
					indexes: null,
				},
			}
		);

		let data = response.data;
		postProcess(data);
		return data;
	}
}
