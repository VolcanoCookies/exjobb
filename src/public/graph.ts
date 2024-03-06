/// <reference path="../../node_modules/@types/d3/index.d.ts" />
import { walkUpBindingElementsAndPatterns } from 'typescript';
import { TrafikverketFlowEntry } from '../model/trafikverketFlowModel.js';

import * as d3Module from 'd3';
declare var d3: typeof d3Module;

const graphOverlay = document.getElementById('graph-overlay') as HTMLDivElement;
const typeSelect = document.getElementById(
	'graph-type-select'
) as HTMLSelectElement;
const startDate = document.getElementById(
	'graph-start-date'
) as HTMLInputElement;
const endDate = document.getElementById('graph-end-date') as HTMLInputElement;

let currentSiteId = 0;

export async function onGraphOpen(siteId: number) {
	if (currentSiteId === 0) {
		startDate.value = new Date(Date.now() - 1000 * 60 * 60 * 24 * 7)
			.toISOString()
			.replace('Z', '');
		endDate.value = new Date().toISOString().replace('Z', '');
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
	await loadGraph(siteId, 'car', start, end);
	showGraph();
}

function onSettingsChange() {
	const type = typeSelect.value || 'car';
	loadGraph(
		currentSiteId,
		type,
		new Date(startDate.value),
		new Date(endDate.value)
	);
}

typeSelect.onchange = onSettingsChange;
startDate.onchange = onSettingsChange;
endDate.onchange = onSettingsChange;

export async function loadGraph(
	siteId: number,
	vehicleType: string,
	start: Date,
	end: Date
) {
	console.log('Loading graph for', siteId, vehicleType, start, end);

	const res = await fetch(`/flow/trafikverket/historic`, {
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
	});

	const data = (await res.json()) as { flows: TrafikverketFlowEntry[] };
	data.flows.forEach((flow) => {
		flow.MeasurementTime = new Date(flow.MeasurementTime);
		flow.ModifiedTime = new Date(flow.ModifiedTime);
	});

	if (data.flows.length === 0) {
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
		.domain(d3.extent(data.flows, (d) => d.MeasurementTime) as [Date, Date])
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
				data.flows,
				(d) => d.VehicleFlowRate / d.MeasurementOrCalculationPeriod
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
		.y((d) => y(d.VehicleFlowRate / d.MeasurementOrCalculationPeriod));

	path.append('path')
		.datum(data.flows)
		.classed('line', true)
		.attr('fill', 'none')
		.attr('stroke', 'steelblue')
		.attr('stroke-width', 2)
		.attr('d', line(data.flows));

	path.append('g').attr('class', 'brush').call(brush);

	var idleTimeout: NodeJS.Timeout | null;
	function idled() {
		idleTimeout = null;
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
		xAxis.transition().duration(1000).call(d3.axisBottom(x));
		path.select('.line')
			.transition()
			.duration(1000)
			.attr('d', line(data.flows));
	}

	// If user double click, reinitialize the chart
	svg.on('dblclick', function () {
		x.domain(
			d3.extent(data.flows, (d) => d.MeasurementTime) as [Date, Date]
		);
		xAxis.transition().call(d3.axisBottom(x));
		path.select('.line').transition().attr('d', line(data.flows));
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
