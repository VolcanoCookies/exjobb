import axios, { AxiosInstance } from 'axios';
import {
	TrafikVerketResponseRaw,
	TrafikVerketRoadGeometry,
	TrafikVerketRoadGeometryReponse,
	TrafikVerketTrafficFlow,
	TrafikVerketTrafficFlowResponse,
} from './types';
import { Point } from '../..';

function getInnerResult<T>(response: TrafikVerketResponseRaw<T>): T[] {
	return response.RESPONSE.RESULT;
}

export class TrafikVerketClient {
	private apiKey: string;
	private client: AxiosInstance;

	constructor(apiKey: string) {
		this.apiKey = apiKey;
		this.client = axios.create();
	}

	async getTrafficFlow(
		limit: number,
		skip: number = 0,
		lastChangedId: number | undefined = undefined,
		near:
			| {
					latitude: number;
					longitude: number;
					radius: number;
			  }
			| undefined = undefined
	): Promise<TrafikVerketTrafficFlowResponse> {
		let filter = '';
		if (near !== undefined) {
			filter = `<FILTER><NEAR name="Geometry.WGS84" value="${near.longitude} ${near.latitude}" mindistance="0" maxdistance="${near.radius}"/></FILTER>`;
		}

		const request = `<REQUEST>
  <LOGIN authenticationkey="${this.apiKey}"/>
  <QUERY objecttype="TrafficFlow" schemaversion="1.4" limit="${limit}" skip="${skip}" ${
			lastChangedId !== undefined ? `changeid="${lastChangedId}"` : ''
		}>
		${filter}
  </QUERY>
</REQUEST>`;

		const response = await this.client.post(
			`https://api.trafikinfo.trafikverket.se/v2/data.json`,
			request,
			{
				headers: {
					'Content-Type': 'text/xml',
				},
			}
		);

		let data = getInnerResult<TrafikVerketTrafficFlowResponse>(
			response.data
		)[0];

		data.TrafficFlow.forEach((flow) => {
			const [lon, lat] = flow.Geometry.WGS84.replace('POINT (', '')
				.replace(')', '')
				.split(' ')
				.map((s) => parseFloat(s));

			flow.Geometry.Point = {
				latitude: lat,
				longitude: lon,
			};
		});

		// @ts-ignore
		data.LastChangeId = data['INFO']?.['LASTCHANGEID'];
		// @ts-ignore
		delete data['INFO'];

		return data;
	}

	async getAllTrafficFlow(
		chunkSize: number = 1000,
		lastChangeId: number | undefined = undefined,
		near:
			| {
					latitude: number;
					longitude: number;
					radius: number;
			  }
			| undefined = undefined
	): Promise<TrafikVerketTrafficFlowResponse> {
		let data: TrafikVerketTrafficFlow[] = [];
		let i = 0;
		let res: TrafikVerketTrafficFlowResponse;
		do {
			res = await this.getTrafficFlow(
				chunkSize,
				i * chunkSize,
				lastChangeId,
				near
			);
			data = data.concat(res.TrafficFlow);
			i++;
		} while (res.TrafficFlow.length === chunkSize);

		return {
			TrafficFlow: data,
			LastChangeId: res.LastChangeId,
		};
	}

	async getRoadGeometry(
		location: Point,
		radius: number,
		limit: number = 10,
		skip: number = 0
	): Promise<TrafikVerketRoadGeometry[]> {
		const filter = `<FILTER><NEAR name="Geometry.WGS843D" value="${location.longitude} ${location.latitude}" mindistance="0" maxdistance="${radius}m"/></FILTER>`;

		const request = `<REQUEST>
  <LOGIN authenticationkey="${this.apiKey}"/>
  <QUERY objecttype="RoadGeometry" schemaversion="1" limit="${limit}" skip="${skip}">
		${filter}
  </QUERY>
</REQUEST>`;

		const response = await this.client.post(
			`https://api.trafikinfo.trafikverket.se/v2/data.json`,
			request,
			{
				headers: {
					'Content-Type': 'text/xml',
				},
			}
		);

		const data = getInnerResult<TrafikVerketRoadGeometryReponse>(
			response.data
		)[0];

		return data.RoadGeometry.map((raw) => {
			const inner = raw.Geometry.WGS843D.replace(
				'LINESTRING (',
				''
			).replace(')', '');
			const coords = inner
				.split(',')
				.map((s) => s.trim().split(' '))
				.map((s) => ({
					latitude: parseFloat(s[1]),
					longitude: parseFloat(s[0]),
				}));

			return {
				County: raw.County,
				Deleted: raw.Deleted,
				Direction: raw.Direction,
				Geometry: {
					Coordinates: coords,
				},
				Length: raw.Length,
				ModifiedTime: new Date(raw.ModifiedTime),
				RoadMainNumber: raw.RoadMainNumber,
				RoadSubNumber: raw.RoadSubNumber,
				TimeStamp: new Date(raw.TimeStamp),
			} as TrafikVerketRoadGeometry;
		});
	}
}
