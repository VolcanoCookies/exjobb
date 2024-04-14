declare module 'geo-nearby' {
	export interface CompactSet {}
	export interface CompactSetEntry {
		i: number;
		g: number;
	}

	export function createCompactSet(data: any, opts: any): CompactSetEntry[];
	export function nearBy(lat: number, lon: number, radius: number): any[];

	export default class Geo {
		constructor(data: any, opts: any);
		static createCompactSet(data: any, opts: any): CompactSetEntry[];
		nearBy(lat: number, lon: number, radius: number): any[];
	}
}
