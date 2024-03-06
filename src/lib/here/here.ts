import { Point } from '../../index.js';
import { distanceMeters } from '../utils.js';
import { HereFlowResponse } from './types.js';

export type MapGraphSegmentId = number;

export class MapGraphSegment {
	id: MapGraphSegmentId;

	start: Point;
	end: Point;

	length: number;

	name: string;

	edges: MapGraphSegmentId[];

	constructor(start: Point, end: Point, length: number, name: string) {
		this.id = -1;
		this.start = start;
		this.end = end;
		this.length = length;
		this.name = name;
		this.edges = [];
	}
}
export class Graph {
	nodes: Map<MapGraphSegmentId, MapGraphSegment>;
	byStart: Map<string, MapGraphSegment[]>;

	constructor() {
		this.nodes = new Map();
		this.byStart = new Map();
	}

	addSegment(segment: MapGraphSegment) {
		let id = this.nodes.size;
		let startKey = segment.start.latitude + ',' + segment.start.longitude;

		const existing = this.byStart.get(startKey);
		if (existing !== undefined) {
			existing.push(segment);
		} else {
			this.byStart.set(startKey, [segment]);
		}

		segment.id = id;
		this.nodes.set(id, segment);
	}

	getSegment(id: MapGraphSegmentId): MapGraphSegment | undefined {
		return this.nodes.get(id);
	}

	getSegments(point: Point): MapGraphSegment[] {
		let key = point.latitude + ',' + point.longitude;
		return this.byStart.get(key) || [];
	}

	forEach(callback: (segment: MapGraphSegment) => void) {
		this.nodes.forEach(callback);
	}

	findClosest(point: Point) {
		let minDistance = Number.MAX_VALUE;
		let closest: MapGraphSegment | undefined = undefined;

		for (const segment of this.nodes.values()) {
			let distance = distanceMeters(
				point.latitude,
				point.longitude,
				segment.start.latitude,
				segment.start.longitude
			);
			if (distance < minDistance) {
				minDistance = distance;
				closest = segment;
			}
		}

		return closest;
	}
}

export function buildGraphFromHereResponse(response: HereFlowResponse): Graph {
	const graph = new Graph();

	for (const result of response.results) {
		for (const link of result.location.shape.links) {
			let p1 = link.points[0];
			let point1 = { latitude: p1.lat, longitude: p1.lng };
			let p2 = link.points[link.points.length - 1];
			let point2 = { latitude: p2.lat, longitude: p2.lng };

			let segment = new MapGraphSegment(
				point1,
				point2,
				link.length,
				result.location.description!
			);

			graph.addSegment(segment);
		}
	}

	graph.forEach((segment) => {
		let neighbours = graph.getSegments(segment.end);
		for (let neighbour of neighbours) {
			segment.edges.push(neighbour.id);
		}
	});

	return graph;
}

export function shortestPathBFS(
	graph: Graph,
	start: Point,
	end: Point
): MapGraphSegment[] | undefined {
	const startSegment = graph.findClosest(start);
	const endSegment = graph.findClosest(end);

	if (startSegment === undefined || endSegment === undefined) {
		return undefined;
	}

	console.log('start', startSegment);
	console.log('end', endSegment);

	interface VisitedNode {
		shortestPath: MapGraphSegment[];
		shortestPathLength: number;
		length: number;
	}

	function insertOrdered(
		array: MapGraphSegment[],
		segment: MapGraphSegment
	): MapGraphSegment[] {
		let index = 0;
		while (index < array.length && array[index].length < segment.length) {
			index++;
		}
		array.splice(index, 0, segment);
		return array;
	}

	const populate = (
		segments: MapGraphSegment[],
		id: MapGraphSegmentId,
		_index: number
	) => {
		const segment = graph.getSegment(id);
		if (segment === undefined) {
			return segments;
		}
		segments.push(segment);
		return segments;
	};

	const visited = new Map<MapGraphSegmentId, VisitedNode>();
	const queue = insertOrdered([], startSegment);

	visited.set(startSegment.id, {
		shortestPath: [],
		shortestPathLength: 0,
		length: startSegment.length,
	});

	console.log('------');

	while (queue.length > 0) {
		const segment = graph.getSegment(queue.shift()!.id);
		if (segment === undefined) {
			continue;
		}

		console.log('segment', segment);

		const visitedNode = visited.get(segment.id);
		if (visitedNode === undefined) {
			continue;
		}

		if (segment.id === endSegment.id) {
			return visitedNode.shortestPath;
		}

		for (const edge of segment.edges) {
			console.log('Getting next segment', edge);

			const nextSegment = graph.getSegment(edge);
			if (nextSegment === undefined) {
				continue;
			}

			const nextVisitedNode = visited.get(nextSegment.id);
			const newLength =
				visitedNode.shortestPathLength + visitedNode.length;

			if (
				nextVisitedNode === undefined ||
				newLength < nextVisitedNode.shortestPathLength
			) {
				visited.set(nextSegment.id, {
					shortestPath: [...visitedNode.shortestPath, nextSegment],
					shortestPathLength: newLength,
					length: nextSegment.length,
				});

				nextSegment.edges.reduce(populate, []).forEach((segment) => {
					insertOrdered(queue, segment);
				});
			}
		}
	}
}
