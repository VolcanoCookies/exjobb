/// <reference path="../../node_modules/@types/d3/index.d.ts" />
import { TrafikverketFlowEntry } from '../model/trafikverketFlowModel.js';

import * as d3Module from 'd3';
import { RouteResponseEntry } from '../model/routeModel.js';
import { BingRouteResponse } from '../lib/bing/types.js';
import { TomTomRouteResponse } from '../lib/tomtom/types.js';
import { HereRouteResponse } from '../lib/here/types.js';
import { BING_COLOR, HERE_COLOR, TOMTOM_COLOR } from './consts.js';
declare var d3: typeof d3Module;

const graphOverlay = document.getElementById('graph-overlay') as HTMLDivElement;
const typeSelect = document.getElementById(
	'graph-type-select'
) as HTMLSelectElement;
const startDate = document.getElementById(
	'graph-start-date'
) as HTMLInputElement;
const endDate = document.getElementById('graph-end-date') as HTMLInputElement;
const routeRange = document.getElementById(
	'graph-route-range'
) as HTMLInputElement;

let currentSiteId = 0;

export async function onGraphOpen(siteId: number) {
	if (currentSiteId === 0) {
		startDate.value = new Date(Date.now() - 1000 * 60 * 60 * 24 * 7)
			.toISOString()
			.replace('Z', '');
		endDate.value = new Date().toISOString().replace('Z', '');
		routeRange.value = '10';
	}
	currentSiteId = siteId;

	fetch(`/flow/trafikverket/vehicleTypes/${siteId}`)
		.then((res) => res.json())
		.then((data) => {
			typeSelect.innerHTML = '';
			const types = data.types as string[];
			types.forEach((type) => {
				const option = document.createElement('option');
				option.value = type;
				option.textContent = type;
				typeSelect.appendChild(option);
			});
		});

	const start = new Date(startDate.value);
	const end = new Date(endDate.value);
	const range = Number(routeRange.value);
	await loadGraph(siteId, 'car', start, end, range);
	showGraph();
}

function onSettingsChange() {
	const type = typeSelect.value || 'car';
	loadGraph(
		currentSiteId,
		type,
		new Date(startDate.value),
		new Date(endDate.value),
		Number(routeRange.value)
	);
}

typeSelect.onchange = onSettingsChange;
startDate.onchange = onSettingsChange;
endDate.onchange = onSettingsChange;
routeRange.onchange = onSettingsChange;

export async function loadGraph(
	siteId: number,
	vehicleType: string,
	start: Date,
	end: Date,
	routeRange: number = 10
) {
	console.log('Loading graph for', siteId, vehicleType, start, end);

	const flowData = (await fetch(`/flow/trafikverket/historic`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
		},
		body: JSON.stringify({
			SiteId: siteId,
			Before: end,
			After: start,
			VehicleType: vehicleType,
		}),
	}).then((res) => res.json())) as { flows: TrafikverketFlowEntry[] };

	flowData.flows.forEach((flow) => {
		flow.MeasurementTime = new Date(flow.MeasurementTime);
		flow.ModifiedTime = new Date(flow.ModifiedTime);
	});

	const latitude =
		flowData.flows.length > 0 ? flowData.flows[0].Point.latitude : 0;
	const longitude =
		flowData.flows.length > 0 ? flowData.flows[0].Point.longitude : 0;

	const routeData = (await fetch(`/routes/inRange`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
		},
		body: JSON.stringify({
			latitude,
			longitude,
			radius: routeRange,
		}),
	}).then((res) => res.json())) as {
		bing: RouteResponseEntry<BingRouteResponse>[];
		tomtom: RouteResponseEntry<TomTomRouteResponse>[];
		here: RouteResponseEntry<HereRouteResponse>[];
	};
	routeData.bing.forEach((route) => {
		route.date = new Date(route.date);
	});
	routeData.tomtom.forEach((route) => {
		route.date = new Date(route.date);
	});
	routeData.here.forEach((route) => {
		route.date = new Date(route.date);
	});

	if (flowData.flows.length === 0) {
		console.log('No data');
		d3.select('#graph > svg').remove();
		return;
	}

	// Clear previous graph
	d3.select('#graph > svg').remove();

	const width = window.innerWidth * 0.8;
	const height = window.innerHeight * 0.8;

	const svg = d3
		.select('#graph')
		.append('svg')
		.attr('width', width)
		.attr('height', height)
		.append('g')
		.attr('transform', 'translate(50, 50)')
		.attr('stroke', 'white');

	const x = d3
		.scaleTime()
		.domain(
			d3.extent(flowData.flows, (d) => d.MeasurementTime) as [Date, Date]
		)
		.range([0, width - 100]);
	const xAxis = svg
		.append('g')
		.attr('transform', `translate(0, ${height - 100})`)
		.call(d3.axisBottom(x))
		.attr('stroke', 'white');

	const y = d3
		.scaleLinear()
		.domain([
			0,
			(d3.max(
				flowData.flows,
				(d) => d.AverageVehicleSpeed
				//(d) => d.VehicleFlowRate / d.MeasurementOrCalculationPeriod
			) as number) * 1.1,
		])
		.range([height - 100, 0]);
	const yAxis = svg.append('g').call(d3.axisLeft(y)).attr('stroke', 'white');

	var clip = svg
		.append('defs')
		.append('svg:clipPath')
		.attr('id', 'clip')
		.append('svg:rect')
		.attr('width', width)
		.attr('height', height)
		.attr('x', 0)
		.attr('y', 0);

	// Add brushing
	var brush = d3
		.brushX()
		.extent([
			[0, 0],
			[width, height],
		])
		.on('end', () => updateChart());

	const path = svg.append('g').attr('clip-path', 'url(#clip)');

	const line = d3
		.line<TrafikverketFlowEntry>()
		.x((d) => x(d.MeasurementTime))
		.y((d) => y(d.AverageVehicleSpeed));
	//.y((d) => y(d.VehicleFlowRate / d.MeasurementOrCalculationPeriod));

	path.append('path')
		.datum(flowData.flows)
		.classed('line', true)
		.attr('fill', 'none')
		.attr('stroke', 'steelblue')
		.attr('stroke-width', 2)
		.attr('d', line(flowData.flows));

	const dots = path.append('g').classed('dot', true);

	const bingData = routeData.bing.map((d) => ({
		date: d.date,
		value: d.response.resourceSets[0].resources[0].travelDurationTraffic,
	}));
	const tomtomData = routeData.tomtom.map((d) => ({
		date: d.date,
		value: d.response.routes[0].summary.travelTimeInSeconds,
	}));
	const hereData = routeData.here.map((d) => ({
		date: d.date,
		value: d.response.routes[0].sections.reduce(
			(a, b) => a + b.summary.duration,
			0
		),
	}));

	function createDots(
		elemClass: string,
		data: { date: Date; value: number }[],
		color: string
	) {
		dots.selectAll(elemClass)
			.data(data)
			.enter()
			.append('circle')
			.classed(elemClass, true)
			.attr('cx', (d) => x(d.date))
			.attr('cy', (d) => y(d.value))
			.attr('r', 2.0)
			.attr('fill', color);
	}

	createDots('bing-dot', bingData, BING_COLOR);
	createDots('tomtom-dot', tomtomData, TOMTOM_COLOR);
	createDots('here-dot', hereData, HERE_COLOR);

	path.append('g').attr('class', 'brush').call(brush);

	var idleTimeout: NodeJS.Timeout | null;
	function idled() {
		idleTimeout = null;
	}

	function updateDots(
		elemClass: string,
		data: { date: Date; value: number }[]
	) {
		dots.selectAll(elemClass)
			.data(data)
			.transition()
			.duration(500)
			.attr('cx', (d) => x(d.date))
			.attr('cy', (d) => y(d.value));
	}

	function updateChart() {
		// What are the selected boundaries?
		// @ts-ignore
		const extent = d3.event.selection;

		// If no selection, back to initial coordinate. Otherwise, update X axis domain
		if (!extent) {
			if (!idleTimeout) return (idleTimeout = setTimeout(idled, 350)); // This allows to wait a little bit
			x.domain([4, 8]);
		} else {
			x.domain([
				x.invert(extent[0] as Number),
				x.invert(extent[1] as Number),
			]);
			// @ts-ignore
			path.select('.brush').call(brush.move, null); // This remove the grey brush area as soon as the selection has been done
		}

		// Update axis and line position
		xAxis.transition().duration(500).call(d3.axisBottom(x));
		path.select('.line')
			.transition()
			.duration(500)
			.attr('d', line(flowData.flows));
		updateDots('.bing-dot', bingData);
		updateDots('.tomtom-dot', tomtomData);
		updateDots('.here-dot', hereData);
	}

	// If user double click, reinitialize the chart
	svg.on('dblclick', function () {
		x.domain(
			d3.extent(flowData.flows, (d) => d.MeasurementTime) as [Date, Date]
		);
		xAxis.transition().call(d3.axisBottom(x));
		path.select('.line')
			.transition()
			.duration(500)
			.attr('d', line(flowData.flows));
		updateDots('.bing-dot', bingData);
		updateDots('.tomtom-dot', tomtomData);
		updateDots('.here-dot', hereData);
	});
}

export function showGraph() {
	graphOverlay.style.display = 'block';
}

export function hideGraph() {
	graphOverlay.style.display = 'none';
}

graphOverlay.onclick = (e) => {
	if (e.target === graphOverlay) {
		hideGraph();
	}
};
