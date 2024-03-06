import { writeFileSync } from 'fs';
import { Point } from '../..';
import { HereFlowResponse } from './types';
import crypto from 'crypto';
import axios, { AxiosInstance } from 'axios';

function postProcess(response: HereFlowResponse) {
	for (const result of response.results) {
		const flow = result.currentFlow;
		if (flow.confidence <= 0.5) {
			flow.confidenceIs = 'speedLimit';
		} else if (flow.confidence <= 0.7) {
			flow.confidenceIs = 'historical';
		} else {
			flow.confidenceIs = 'realtime';
		}

		const hash = crypto.createHash('sha256');

		if (result.location.description !== undefined) {
			hash.update(result.location.description);
		}
		hash.update(result.location.length.toString());

		result.location.shape.links.forEach((link) => {
			link.points.forEach((point) => {
				hash.update(`${point.lat},${point.lng}`);
			});
		});

		result.location.hash = hash.digest('hex');
	}
}

export class HereClient {
	private apiKey: string;
	private client: AxiosInstance;

	constructor(apiKey: string) {
		this.apiKey = apiKey;
		this.client = axios.create();
	}

	async getFlow(
		latitude: number,
		longitude: number,
		radius: number
	): Promise<HereFlowResponse> {
		if (radius > 50000) {
			throw new Error('Max radius is 50000');
		}

		const response = await this.client.get(
			`https://data.traffic.hereapi.com/v7/flow?locationReferencing=shape&in=circle:${latitude},${longitude};r=${radius}&apiKey=${this.apiKey}`
		);

		writeFileSync('here.json', JSON.stringify(response.data, null, 2));

		let data = response.data;
		postProcess(data);
		return data;
	}

	async getRoute(
		start: Point,
		end: Point,
		via: Point[] = [],
		heading: number | undefined = undefined
	) {
		let origin = `${start.latitude},${start.longitude}`;
		if (heading !== undefined) {
			origin += `;course=${heading.toFixed(0)}`;
		}

		const response = await this.client.get(
			`https://router.hereapi.com/v8/routes`,
			{
				params: {
					transportMode: 'car',
					origin: origin,
					destination: `${end.latitude},${end.longitude}`,
					via: via.map(
						(point) => `${point.latitude},${point.longitude}`
					),
					return: 'summary,polyline,passthrough',
					spans: 'length,duration,baseDuration,typicalDuration,maxSpeed,dynamicSpeedInfo,segmentRef,carAttributes',
					apiKey: this.apiKey,
				},
				paramsSerializer: {
					indexes: null,
				},
			}
		);

		let data = response.data;
		return data;
	}
}
