// @ts-ignore
import * as d3 from 'https://cdn.jsdelivr.net/npm/d3@7/+esm';
import {
	TrafikVerketRoadGeometry,
	TrafikVerketTrafficFlowResponse,
} from '../lib/trafikverket/types.js';

const width = 1280;
const height = 800;

// Create the SVG container.
const svg = d3.create('svg').attr('width', width).attr('height', height);
const g = svg.append('g');

const roadData = (await fetch('/geometry').then((res) =>
	res.json()
)) as TrafikVerketRoadGeometry[];

const sensorData = (
	(await fetch('/flow/trafikverket').then((res) =>
		res.json()
	)) as TrafikVerketTrafficFlowResponse
).TrafficFlow;

let minLat = Infinity;
let maxLat = -Infinity;
let minLng = Infinity;
let maxLng = -Infinity;

for (const res of roadData) {
	for (const point of res.Geometry.Coordinates) {
		const coords = fromLongLat(point.longitude, point.latitude);
		point.latitude = coords.x;
		point.longitude = coords.y;
		minLat = Math.min(minLat, point.latitude);
		maxLat = Math.max(maxLat, point.latitude);
		minLng = Math.min(minLng, point.longitude);
		maxLng = Math.max(maxLng, point.longitude);
	}
}

for (const res of sensorData) {
	const point = res.Geometry.Point;
	const coords = fromLongLat(point.longitude, point.latitude);
	point.latitude = coords.x;
	point.longitude = coords.y;
	minLat = Math.min(minLat, point.latitude);
	maxLat = Math.max(maxLat, point.latitude);
	minLng = Math.min(minLng, point.longitude);
	maxLng = Math.max(maxLng, point.longitude);
}

function transformPoint(point: { latitude: number; longitude: number }) {
	const x = ((point.longitude - minLng) / (maxLng - minLng)) * width;
	const y = ((point.latitude - minLat) / (maxLat - minLat)) * height;
	return { x, y };
}

function getColor(speed: number) {
	// Speed between 0 and 1, red for 0, green for 1, interpolate between
	const r = Math.floor(255 * (1 - speed));
	const g = Math.floor(255 * speed);
	if (speed === 1) {
		return 'rgb(0,0,255)';
	}
	return `rgb(${r},${g},0)`;
}

const line = d3
	.line<{ lat: number; lng: number }>()
	.x((d: { x: number }) => d.x)
	.y((d: { y: number }) => d.y);

for (const res of roadData) {
	const coords = [];
	for (const point of res.Geometry.Coordinates) {
		const c = transformPoint(point);
		coords.push(c);
	}

	//let speed = res.currentFlow.speedUncapped / res.currentFlow.freeFlow;
	//speed = Math.min(speed, 1);
	const speed = 1;

	g.append('path')
		.classed('line', true)
		.attr('d', line(coords))
		.attr('fill', 'none')
		.attr('stroke', getColor(speed))
		.attr('stroke-linecap', 'round');
}

for (const res of sensorData) {
	const point = res.Geometry.Point;
	const c = transformPoint(point);

	g.append('circle')
		.classed('circle', true)
		.attr('cx', c.x)
		.attr('cy', c.y)
		.attr('r', 5)
		.attr('fill', 'red');
}

function zoomed(e: any) {
	g.attr('transform', e.transform);
	g.selectAll('.circle').attr('r', 5 / e.transform.k);
}

const zoom = d3.zoom().scaleExtent([0.5, 32]).on('zoom', zoomed);

svg.call(zoom);

// Append the SVG element.
const container = document.getElementById('container');
// @ts-ignore
container.append(svg.node());

function fromLongLat(long: number, lat: number) {
	const x = (long + 180) * (width / 360);
	const y = (-1 * lat + 90) * (height / 180);
	return { x, y };
}
