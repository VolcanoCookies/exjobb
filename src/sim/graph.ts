import { configDotenv } from 'dotenv';
import { TrafikVerketClient } from '../lib/trafikverket/client.js';
import { readFile, writeFile } from 'fs/promises';
import { TrafikVerketRoadGeometry } from '../lib/trafikverket/types.js';
import { encodePNGToStream, make } from 'pureimage';
import { createWriteStream } from 'fs';

class Vertex {
	id: number;
	latitude: number;
	longitude: number;

	averageSpeed: number;

	siteIds: number[] = [];

	constructor(
		id: number,
		latitude: number,
		longitude: number,
		averageSpeed: number
	) {
		this.id = id;
		this.latitude = latitude;
		this.longitude = longitude;
		this.averageSpeed = averageSpeed;
	}
}

class Edge {
	id: number;
	source: Vertex;
	target: Vertex;

	constructor(id: number, source: Vertex, target: Vertex) {
		this.id = id;
		this.source = source;
		this.target = target;
	}
}

class Graph {
	vertices: Vertex[];
	edges: Edge[];

	constructor() {
		this.vertices = [];
		this.edges = [];
	}

	addVertex(vertex: Vertex) {
		this.vertices.push(vertex);
	}

	addEdge(edge: Edge) {
		this.edges.push(edge);
	}
}

async function getRoadData() {
	configDotenv();
	const token = process.env.TRAFIKVERKET_API_KEY!;
	const client = new TrafikVerketClient(token);

	const point = {
		latitude: 59.3293,
		longitude: 18.0686,
	};
	return await client.getRoadGeometry(point, 75 * 1000, 1000000);
}

async function test() {
	const raw = await readFile('roadData.json', 'utf-8');
	const data = JSON.parse(raw) as TrafikVerketRoadGeometry[];

	let minLat = Number.POSITIVE_INFINITY;
	let maxLat = Number.NEGATIVE_INFINITY;
	let minLon = Number.POSITIVE_INFINITY;
	let maxLon = Number.NEGATIVE_INFINITY;

	for (const road of data) {
		for (const point of road.Geometry.Coordinates) {
			minLat = Math.min(minLat, point.latitude);
			maxLat = Math.max(maxLat, point.latitude);
			minLon = Math.min(minLon, point.longitude);
			maxLon = Math.max(maxLon, point.longitude);
		}
	}

	const angleWidth = maxLon - minLon;
	const angleHeight = maxLat - minLat;
	// Make width always 2000 and scale height accordingly
	const scale = 2000 / angleWidth;
	const width = 2000;
	const height = Math.round(angleHeight * scale);

	function convert(point: { latitude: number; longitude: number }) {
		const x = Math.round((point.longitude - minLon) * scale);
		const y = Math.round((point.latitude - minLat) * scale);
		return { x, y };
	}

	console.log('Width: ', width);
	console.log('Height: ', height);

	const image = make(width, height);
	const ctx = image.getContext('2d');

	ctx.fillStyle = '#1f1f1f';
	ctx.fillRect(0, 0, width, height);

	ctx.strokeStyle = '#ffffff';
	ctx.lineWidth = 2;

	let i = 0;
	for (const road of data) {
		let { x, y } = convert(road.Geometry.Coordinates[0]);
		ctx.beginPath();
		ctx.moveTo(x, y);
		let px = x;
		let py = y;
		for (let i = 1; i < road.Geometry.Coordinates.length; i++) {
			let { x, y } = convert(road.Geometry.Coordinates[i]);
			if (x === px && y === py) continue;
			ctx.lineTo(x, y);

			px = x;
			py = y;
		}
		console.log(
			'Stroking road of length ' + road.Geometry.Coordinates.length
		);
		ctx.stroke();

		i++;

		console.log('Done ' + i + ' / ' + data.length);
	}
	console.log('Done drawing, saving image');

	encodePNGToStream(image, createWriteStream('image.png'));
}

export default test();
