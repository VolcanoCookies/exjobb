// @ts-ignore
import * as d3 from 'https://cdn.jsdelivr.net/npm/d3@7/+esm';
import { HereFlowResponse } from '../lib/here/types.js';

const width = 1280;
const height = 800;

// Create the SVG container.
const svg = d3.create('svg').attr('width', width).attr('height', height);
const g = svg.append('g');

const data: HereFlowResponse = await fetch('/data').then((res) => res.json());

let minLat = Infinity;
let maxLat = -Infinity;
let minLng = Infinity;
let maxLng = -Infinity;

for (const res of data.results) {
	for (const link of res.location.shape.links) {
		for (const point of link.points) {
			const coords = fromLongLat(point.lng, point.lat);
			point.lat = coords.x;
			point.lng = coords.y;
			minLat = Math.min(minLat, point.lat);
			maxLat = Math.max(maxLat, point.lat);
			minLng = Math.min(minLng, point.lng);
			maxLng = Math.max(maxLng, point.lng);
		}
	}
}

function transformPoint(point: { lat: number; lng: number }) {
	const x = ((point.lng - minLng) / (maxLng - minLng)) * width;
	const y = ((point.lat - minLat) / (maxLat - minLat)) * height;
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

for (const res of data.results) {
	for (const link of res.location.shape.links) {
		const coords = [];
		for (const point of link.points) {
			const c = transformPoint(point);
			coords.push(c);
		}

		let speed = res.currentFlow.speedUncapped / res.currentFlow.freeFlow;
		speed = Math.min(speed, 1);

		g.append('path')
			.classed('line', true)
			.attr('d', line(coords))
			.attr('fill', 'none')
			.attr('stroke', getColor(speed))
			.attr('stroke-linecap', 'round');
	}
}

function zoomed(e: any) {
	g.attr('transform', e.transform);
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
