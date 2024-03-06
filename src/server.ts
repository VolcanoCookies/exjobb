import express from 'express';
import dotenv from 'dotenv';
import { readDirRecursiveSync } from './lib/utils.js';
import { Point } from './index.js';
import { BingClient } from './lib/bing/client.js';
import { HereClient } from './lib/here/client.js';
import { TomTomClient } from './lib/tomtom/client.js';
import { get_bearing, save_route } from './utils.js';
import bodyParser from 'body-parser';
import { TrafikVerketClient } from './lib/trafikverket/client.js';
import { writeFileSync } from 'fs';
import mongoose from 'mongoose';
import { TrafikverketFlowEntryModel } from './model/trafikverketFlowModel.js';

dotenv.config();

const mongoUrl = process.env.MONGO_URL!;
if (!mongoUrl) {
	throw new Error('MONGO_URL not found');
}

mongoose
	.connect(mongoUrl, {
		dbName: 'exjobb',
	})
	.then(() => {
		console.log('Connected to MongoDB');
	});

const bindApiKey = process.env.BING_API_KEY!;
const hereApiKey = process.env.HERE_API_KEY!;
const tomtomApiKey = process.env.TOMTOM_API_KEY!;
const trafikverketApiKey = process.env.TRAFIKVERKET_API_KEY!;

const bingClient = new BingClient(bindApiKey);
const hereClient = new HereClient(hereApiKey);
const tomtomClient = new TomTomClient(tomtomApiKey);
const trafikverketClient = new TrafikVerketClient(trafikverketApiKey);

const __dirname = import.meta.dirname;

const app = express();
app.use(bodyParser.json());
const port = 3000;

app.use('/', express.static(__dirname + '/public'));
app.use('/lib', express.static(__dirname + '/lib'));

app.get('/', (req, res) => {
	res.sendFile(__dirname + './public/index.html');
});

app.use('/data', express.static(__dirname + '/../data'));

/*
app.get('/data', (req, res) => {
	const roads = new Map<string, HereResult[]>();

	for (let i = 0; i < 36; i++) {
		const data: HereFlowResponse = JSON.parse(
			readFileSync(
				__dirname + `/../data/2024-02-08/here_${i}.json`,
				'utf-8'
			)
		);

		data.results.forEach((result) => {
			const hash = result.location.hash;
			if (roads.has(hash)) {
				const existing = roads.get(hash);
				if (existing) {
					existing.push(result);
				} else {
					roads.set(hash, [result]);
				}
			} else {
				roads.set(hash, [result]);
			}
		});
	}

	const result: HereResult[] = [];
	roads.forEach((value) => {
		let flow: HereFlow = {
			speed: 0,
			speedUncapped: 0,
			freeFlow: 0,
			jamFactor: 0,
			confidence: 0,
			traversability: 'open',
			confidenceIs: 'realtime',
		};

		for (const v of value) {
			flow.speed += v.currentFlow.speed;
			flow.speedUncapped += v.currentFlow.speedUncapped;
			flow.freeFlow += v.currentFlow.freeFlow;
			flow.jamFactor += v.currentFlow.jamFactor;
			flow.confidence += v.currentFlow.confidence;
		}

		flow.speed /= value.length;
		flow.speedUncapped /= value.length;
		flow.freeFlow /= value.length;
		flow.jamFactor /= value.length;
		flow.confidence /= value.length;

		result.push({
			location: value[0].location,
			currentFlow: flow,
		});
	});

	const data: HereFlowResponse = {
		sourceUpdated: new Date().toISOString(),
		results: result,
	};

	res.json(data);
});
*/

app.get('/routes/list', (req, res) => {
	const parent = __dirname + '/../data/routes/';
	const files = readDirRecursiveSync(parent);

	return res.json({
		files: files.map((f) => 'routes/' + f.replace(parent, '')),
	});
});

interface CompareRequest {
	points: Point[];
	name: string;
	times: number | undefined;
}
app.post<CompareRequest>('/routes/compare', async (req, res) => {
	const data = req.body as CompareRequest;
	if (data.points === undefined || data.points.length < 2) {
		return res.status(400).send('At least two points are required');
	} else if (data.points.length > 25) {
		return res.status(400).send('Maximum 25 points are allowed');
	} else if (data.name === undefined || data.name.trim() === '') {
		return res.status(400).send('Name is required');
	}

	const bearing = Math.round(get_bearing(data.points[0], data.points[1]));

	const now = Math.round(new Date().getTime() / 1000);
	const bingRoute = bingClient
		.getRoute(data.points, bearing)
		.then((route) => save_route(route, `${data.name}/${now}-bing.json`));
	const hereRoute = hereClient
		.getRoute(
			data.points[0],
			data.points[data.points.length - 1],
			data.points.slice(1, -1),
			bearing
		)
		.then((route) => save_route(route, `${data.name}/${now}-here.json`));
	const tomtomRoute = tomtomClient
		.getRoute(data.points, bearing)
		.then((route) => save_route(route, `${data.name}/${now}-tomtom.json`));

	const [bing, here, tomtom] = await Promise.all([
		bingRoute,
		hereRoute,
		tomtomRoute,
	]);

	return res.json({
		bing,
		here,
		tomtom,
	});
});

app.get('/flow/trafikverket', async (req, res) => {
	const data = await trafikverketClient
		.getAllTrafficFlow(10000, undefined, {
			latitude: 59.325484,
			longitude: 18.0653,
			radius: 100 * 1000,
		})
		.catch((e) => {
			writeFileSync('error.json', JSON.stringify(e, null, 2));
			throw e;
		});
	return res.json(data);
});

interface FlowRequest {
	SiteId: number;
	Before: Date;
	After: Date;
	VehicleType: string | undefined;
}
app.post<FlowRequest>('/flow/trafikverket/historic', async (req, res) => {
	const data = req.body as FlowRequest;
	if (data.Before === undefined || data.After === undefined) {
		return res.status(400).send('Before and After are required');
	} else if (data.SiteId === undefined) {
		return res.status(400).send('SiteId is required');
	}

	const query = {
		SiteId: data.SiteId,
		MeasurementTime: {
			$gte: data.After,
			$lte: data.Before,
		},
	};
	if (data.VehicleType) {
		// @ts-ignore
		query['VehicleType'] = data.VehicleType;
	}

	const flows = await TrafikverketFlowEntryModel.find(query).exec();
	return res.json({ flows });
});

app.get('/flow/trafikverket/vehicleTypes/:siteId', async (req, res) => {
	const siteId = req.params.siteId;
	const types = await TrafikverketFlowEntryModel.find({
		SiteId: siteId,
	})
		.distinct('VehicleType')
		.exec();

	return res.json({ types });
});

app.listen(port, () => {
	console.log(`Server listening on http://localhost:${port}`);
});
