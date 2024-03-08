/// <reference path="../../node_modules/@types/bingmaps/index.d.ts" />

import { BingRouteResponse, bingRouteStats } from '../lib/bing/types.js';
import { HereRouteResponse, hereRouteStats } from '../lib/here/types.js';
import { TomTomRouteResponse, tomtomRouteStats } from '../lib/tomtom/types.js';
import {
	TrafikVerketTrafficFlow,
	TrafikVerketTrafficFlowResponse,
} from '../lib/trafikverket/types.js';
import { BING_COLOR, HERE_COLOR, TOMTOM_COLOR } from './consts.js';
import { loadGraph, onGraphOpen, showGraph } from './graph.js';

let modeSelect: HTMLSelectElement = document.getElementById(
	'mode-select'
)! as HTMLSelectElement;

let routeSelect: HTMLSelectElement = document.getElementById(
	'route-select'
)! as HTMLSelectElement;

let routeForm: HTMLFormElement = document.getElementById(
	'route-form'
)! as HTMLFormElement;

let routeName: HTMLInputElement = document.getElementById(
	'route-form-name'
)! as HTMLInputElement;

let routeSubmit: HTMLInputElement = document.getElementById(
	'route-form-submit'
)! as HTMLInputElement;

let ctrl_down = false;
addEventListener('keydown', (e) => {
	if (e.key === 'Control') {
		ctrl_down = true;
	}
});
addEventListener('keyup', (e) => {
	if (e.key === 'Control') {
		ctrl_down = false;
	}
});

function canSubmitForm() {
	const locations = (
		map.entities.get(0) as Microsoft.Maps.Polyline
	).getLocations().length;
	const canSubmit =
		routeName.value !== '' &&
		routeName.value.trim().length > 0 &&
		locations >= 2;
	routeSubmit.disabled = !canSubmit;
}

routeName.oninput = canSubmitForm;

routeForm.onsubmit = async (e) => {
	e.preventDefault();
	const name = routeName.value;
	const points = (map.entities.get(0) as Microsoft.Maps.Polyline)
		.getLocations()
		.map((l) => {
			return { latitude: l.latitude, longitude: l.longitude };
		});

	const res = await fetch('/routes/compare', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
		},
		body: JSON.stringify({ name, points }),
	});

	const data = await res.json();
	const paths = [data['bing'], data['here'], data['tomtom']].map((r) =>
		r.replace('/data/', '')
	);
	await loadRoutes(paths);
};

let map: Microsoft.Maps.Map;
function loadMap() {
	console.log('Loading map');

	map = new Microsoft.Maps.Map('#myMap', {
		credentials:
			'***REMOVED***',
	});

	Microsoft.Maps.loadModule('Microsoft.Maps.Clustering', function () {
		loadFlowPoints();
	});

	map.entities.add(
		new Microsoft.Maps.Polyline([new Microsoft.Maps.Location(0, 0)], {
			strokeColor: 'blue',
		}),
		0
	);
	let initialized = false;
	Microsoft.Maps.Events.addHandler(map, 'mousedown', (e) => {
		if (e === undefined) return;
		let event = e as Microsoft.Maps.IMouseEventArgs;

		if (
			event.location === undefined ||
			event.isPrimary === true ||
			event.isSecondary === false
		)
			return;

		if (!initialized) {
			initialized = true;
			map.entities.add(
				new Microsoft.Maps.Polyline([], {
					strokeColor: 'blue',
					strokeThickness: 3,
				}),
				0
			);
		}
		let polyline = map.entities.get(0) as Microsoft.Maps.Polyline;
		let locations = polyline.getLocations();
		if (ctrl_down) {
			locations.push(event.location);
		} else {
			locations = [event.location];
		}
		polyline.setLocations(locations);

		canSubmitForm();

		// Copy to clipboard
		let text = locations
			.map((l) => `{latitude:${l.latitude},longitude:${l.longitude}}`)
			.join(',');
		navigator.clipboard.writeText(text);
	});

	canSubmitForm();
}
window.onload = loadMap;

async function listRouteFiles(): Promise<string[]> {
	const res = await fetch('/routes/list');
	const data = await res.json();
	return (await data['files']).map((f: string) => f.replaceAll(/\/+/g, '/'));
}

async function loadRoutes(files: string[]) {
	const folder = files[0].substring(0, files[0].lastIndexOf('/') + 1);

	const bingRoutes = await Promise.all(
		files
			.filter((f) => f.includes('bing'))
			.map(async (f) => {
				const res = await fetch(`/data/${f}`);
				const data = (await res.json()) as BingRouteResponse;
				return {
					file: f,
					data,
					stats: bingRouteStats(data),
				};
			})
	);
	const hereRoutes = await Promise.all(
		files
			.filter((f) => f.includes('here'))
			.map(async (f) => {
				const res = await fetch(`/data/${f}`);
				const data = (await res.json()) as HereRouteResponse;
				return {
					file: f,
					data,
					stats: hereRouteStats(data),
				};
			})
	);
	const tomtomRoutes = await Promise.all(
		files
			.filter((f) => f.includes('tomtom'))
			.map(async (f) => {
				const res = await fetch(`/data/${f}`);
				const data = (await res.json()) as TomTomRouteResponse;
				return {
					file: f,
					data,
					stats: tomtomRouteStats(data),
				};
			})
	);

	const statsTable = document.getElementById('route-stats')!;

	const bingRows = bingRoutes.map((r) => {
		return `
		<tr>
			<td>${r.file.replace(folder, '')}</td>
			<td>Bing</td>
			<td>${r.stats.distance}</td>
			<td>${r.stats.duration}</td>
			<td>${r.stats.durationTraffic}</td>
		</tr>
		`;
	});
	const hereRows = hereRoutes.map((r) => {
		return `
		<tr>
			<td>${r.file.replace(folder, '')}</td>
			<td>Here</td>
			<td>${r.stats.distance}</td>
			<td>${r.stats.duration}</td>
			<td>${r.stats.durationTraffic}</td>
		</tr>
		`;
	});
	const tomtomRows = tomtomRoutes.map((r) => {
		return `
		<tr>
			<td>${r.file.replace(folder, '')}</td>
			<td>TomTom</td>
			<td>${r.stats.distance}</td>
			<td>${r.stats.duration}</td>
			<td>${r.stats.durationTraffic}</td>
		</tr>
		`;
	});

	statsTable.innerHTML = `
		<tr>
			<th>Route</th>
			<th>Provider</th>
			<th>Length</th>
			<th>Duration</th>
			<th>Duration Traffic</th>
		</tr>
		${bingRows.join('')}
		${hereRows.join('')}
		${tomtomRows.join('')}
	`;

	const initialPoints = bingRoutes[0].stats.points;

	bingRoutes.forEach((r) => {
		const points = r.stats.points.map((c) => {
			return new Microsoft.Maps.Location(c.latitude, c.longitude);
		});

		const line = new Microsoft.Maps.Polyline(points, {
			strokeColor: BING_COLOR,
			strokeThickness: 2,
		});
		map.entities.add(line);
	});

	hereRoutes.forEach((r) => {
		const points = r.stats.points.map((c) => {
			return new Microsoft.Maps.Location(c.latitude, c.longitude);
		});

		const line = new Microsoft.Maps.Polyline(points, {
			strokeColor: HERE_COLOR,
			strokeThickness: 2,
		});
		map.entities.add(line);
	});

	tomtomRoutes.forEach((r) => {
		const points = r.stats.points.map((c) => {
			return new Microsoft.Maps.Location(c.latitude, c.longitude);
		});

		const line = new Microsoft.Maps.Polyline(points, {
			strokeColor: TOMTOM_COLOR,
			strokeThickness: 2,
		});
		map.entities.add(line);
	});

	map.setView({
		bounds: Microsoft.Maps.LocationRect.fromLocations(
			initialPoints.map((c) => {
				return new Microsoft.Maps.Location(c.latitude, c.longitude);
			})
		),
	});
}

const refreshButton = document.getElementById(
	'refresh-flow'
)! as HTMLButtonElement;
refreshButton.onclick = async () => {
	await loadFlowPoints();
};

let clusterLayer: Microsoft.Maps.ClusterLayer;
async function loadFlowPoints() {
	const res = await fetch('/flow/trafikverket');
	const data = (await res.json()) as TrafikVerketTrafficFlowResponse;

	if (clusterLayer !== undefined) {
		map.layers.remove(clusterLayer);
	}

	const pins = data.TrafficFlow.map((f) => {
		const pin = new Microsoft.Maps.Pushpin(
			new Microsoft.Maps.Location(
				f.Geometry.Point.latitude,
				f.Geometry.Point.longitude
			),
			{
				color: 'red',
			}
		);
		// @ts-ignore
		pin.data = f;

		return pin;
	});

	clusterLayer = new Microsoft.Maps.ClusterLayer(pins, {
		gridSize: 32,
	});
	map.layers.insert(clusterLayer);

	const tooltip = new Microsoft.Maps.Infobox(
		new Microsoft.Maps.Location(0, 0),
		{
			visible: false,
			showPointer: false,
			showCloseButton: false,
		}
	);
	tooltip.setMap(map);

	Microsoft.Maps.Events.addHandler(clusterLayer, 'mouseover', (e) => {
		const target = e['target'];
		const pins = target['containedPushpins'];
		if (pins === undefined || pins.length === 0) return;

		const lanes: {
			[key: string]: {
				types: {
					[key: string]: {
						count: number;
						averageSpeed: number;
					};
				};
				count: number;
				averageSpeed: number;
			};
		} = {};
		for (const pin of pins) {
			const data = pin['data'] as TrafikVerketTrafficFlow;
			const periodModifier = 60 / data.MeasurementOrCalculationPeriod;
			const lane = lanes[data.SpecificLane] || {
				types: {},
				count: 0,
				averageSpeed: 0,
			};
			lane.count += data.VehicleFlowRate * periodModifier;
			lane.averageSpeed +=
				data.AverageVehicleSpeed * data.VehicleFlowRate;

			const type = lane.types[data.VehicleType] || {
				count: 0,
				averageSpeed: 0,
			};
			type.count += data.VehicleFlowRate * periodModifier;
			type.averageSpeed +=
				data.AverageVehicleSpeed * data.VehicleFlowRate;
			lane.types[data.VehicleType] = type;
			lanes[data.SpecificLane] = lane;

			console.log(data.SiteId);
		}

		let table = `
		<div class="flow-table">
		<table>
			<tr>
				<th>Lane</th>
				<th>Type</th>
				<th>Count</th>
				<th>Speed</th>
			</tr>`;

		for (const laneName in lanes) {
			const lane = lanes[laneName];
			lane.averageSpeed /= lane.count;

			for (const typeName in lane.types) {
				const type = lane.types[typeName];
				type.averageSpeed /= type.count;

				table += `<tr>
					<td>${laneName}</td>
					<td>${typeName}</td>
					<td>${type.count.toFixed(0)}</td>
					<td>${type.averageSpeed.toFixed(0)}</td>
				</tr>`;
			}
		}

		table += '</table></div>';

		tooltip.setOptions({
			visible: true,
			title: 'Traffic Flow',
			htmlContent: table,
			location: pins[0].getLocation(),
		});
	});

	Microsoft.Maps.Events.addHandler(clusterLayer, 'mouseout', () => {
		tooltip.setOptions({ visible: false });
	});

	Microsoft.Maps.Events.addHandler(clusterLayer, 'click', (e) => {
		const target = e['target'];
		const pins = target['containedPushpins'];
		if (pins === undefined || pins.length === 0) return;

		const siteId = pins[0]['data']['SiteId'];
		onGraphOpen(siteId);
		showGraph();
	});
}

modeSelect.onchange = async () => {
	const mode = modeSelect.selectedOptions[0].value;
	console.log(mode);
	if (mode === 'route') {
		const files = await listRouteFiles();
		const folders = Array.from(
			new Set(files.map((f) => f.substring(0, f.lastIndexOf('/'))))
		);

		routeSelect.innerHTML = folders
			.map((f) => {
				return `<option value="${f}">${f}</option>`;
			})
			.join('');

		routeSelect.hidden = false;
	} else {
		routeSelect.hidden = true;
	}
};

routeSelect.onchange = async () => {
	const path = routeSelect.selectedOptions[0].value;

	console.log(`Selected path: ${path}`);

	map.entities.clear();

	const files = await listRouteFiles();
	const routes = files.filter((f) => f.includes(path));

	await loadRoutes(routes);
};
