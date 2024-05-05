import { configDotenv } from 'dotenv';
import { sleep } from '../lib/utils.js';
import { connectDb } from '../db.js';
import { BingClient } from '../lib/bing/client.js';
import { TomTomClient } from '../lib/tomtom/client.js';
import { HereClient } from '../lib/here/client.js';
import { Point } from '../index.js';
import { get_bearing } from '../utils.js';
import {
	BingRouteEntryModel,
	TomTomRouteEntryModel,
	HereRouteEntryModel,
} from '../model/routeModel.js';
import { Logger } from '@pieropatron/tinylogger';
import { AxiosError } from 'axios';
import fs from 'fs';
import { group } from 'd3';

configDotenv();
const logger = new Logger('generate-statistics');

interface UnifiedData {
	travelTime: number;
	travelTimeTraffic: number;
	distance: number;
	time: Date;
}

interface GroupedData {
	bing: UnifiedData;
	tomtom: UnifiedData;
	here: UnifiedData;
}

async function main() {
	await connectDb();

	const bingEntries = await BingRouteEntryModel.find();
	const tomtomEntries = await TomTomRouteEntryModel.find();
	const hereEntries = await HereRouteEntryModel.find();

	const data: Map<number, GroupedData> = new Map();

	for (const entry of bingEntries) {
		const resource = entry.response.resourceSets[0].resources[0];

		const travelTime = resource.travelDuration;
		const travelTimeTraffic = resource.travelDurationTraffic;
		const distance = resource.travelDistance * 1000;
		const time = entry.date;

		const key = time.getTime();

		if (!data.has(key)) {
			data.set(key, {} as GroupedData);
		}

		data.get(key)!.bing = {
			travelTime,
			travelTimeTraffic,
			distance,
			time,
		};
	}

	for (const entry of tomtomEntries) {
		const resource = entry.response.routes[0].summary;

		const travelTime = resource.noTrafficTravelTimeInSeconds;
		const travelTimeTraffic =
			resource.liveTrafficIncidentsTravelTimeInSeconds;
		const distance = resource.lengthInMeters;
		const time = entry.date;

		const key = time.getTime();

		if (!data.has(key)) {
			data.set(key, {} as GroupedData);
		}

		data.get(key)!.tomtom = {
			travelTime,
			travelTimeTraffic,
			distance,
			time,
		};
	}

	for (const entry of hereEntries) {
		const section = entry.response.routes[0].sections[0];

		const travelTime = section.summary.baseDuration;
		const travelTimeTraffic = section.summary.duration;
		const distance = section.summary.length;
		const time = entry.date;

		const calculatedTravelTime = section.spans.reduce(
			(acc, span) => acc + span.baseDuration,
			0
		);
		const calculatedTravelTimeTraffic = section.spans.reduce(
			(acc, span) => acc + span.duration,
			0
		);

		if (calculatedTravelTime !== travelTime) {
			logger.warn(
				`Travel time mismatch: ${calculatedTravelTime} !== ${travelTime}`
			);
		}
		if (calculatedTravelTimeTraffic !== travelTimeTraffic) {
			logger.warn(
				`Travel time traffic mismatch: ${calculatedTravelTimeTraffic} !== ${travelTimeTraffic}`
			);
		}

		const key = time.getTime();

		if (!data.has(key)) {
			data.set(key, {} as GroupedData);
		}

		data.get(key)!.here = {
			travelTime,
			travelTimeTraffic,
			distance,
			time,
		};
	}

	const groupedData = Array.from(data.values());

	const groupedDataJson = JSON.stringify(groupedData);
	fs.writeFileSync('groupedData.json', groupedDataJson);

	const data1k = groupedData.filter(
		(entry) => entry.bing.distance <= 1500 && entry.bing.distance >= 500
	);
	const data2k = groupedData.filter(
		(entry) => entry.bing.distance <= 2500 && entry.bing.distance >= 1500
	);
	const data4k = groupedData.filter(
		(entry) => entry.bing.distance <= 4500 && entry.bing.distance >= 3500
	);
	const data8k = groupedData.filter(
		(entry) => entry.bing.distance <= 8500 && entry.bing.distance >= 7500
	);

	writeToFile(data1k, 'routeStats1km.csv');
	writeToFile(data2k, 'routeStats2km.csv');
	writeToFile(data4k, 'routeStats4km.csv');
	writeToFile(data8k, 'routeStats8km.csv');

	logger.info('Done');
}

function formatDate(date: Date) {
	return `${date.getFullYear()}-${zeroPrepad(
		date.getMonth() + 1,
		2
	)}-${zeroPrepad(date.getDate(), 2)} ${zeroPrepad(
		date.getHours(),
		2
	)}:${zeroPrepad(date.getMinutes(), 2)}:${zeroPrepad(date.getSeconds(), 2)}`;
}

function zeroPrepad(num: number, length: number) {
	return num.toString().padStart(length, '0');
}

function writeToFile(data: GroupedData[], filePath: string) {
	const writeStream = fs.createWriteStream(filePath, {
		flags: 'w',
	});

	writeStream.write(
		'travelTimeBing,travelTimeTrafficBing,travelTimeTomTom,travelTimeTrafficTomtom,travelTimeHere,travelTimeTrafficHere,distance,time\n'
	);

	for (const entry of data) {
		writeStream.write(
			`${entry.bing.travelTime},${entry.bing.travelTimeTraffic},${
				entry.tomtom.travelTime
			},${entry.tomtom.travelTimeTraffic},${entry.here.travelTime},${
				entry.here.travelTimeTraffic
			},${entry.bing.distance},${formatDate(entry.bing.time)}\n`
		);
	}

	writeStream.end();
}

export default main();
