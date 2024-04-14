import { TrafikVerketRoadGeometry } from '../lib/trafikverket/types.js';

const data = (await fetch('/geometry').then((res) =>
	res.json()
)) as TrafikVerketRoadGeometry[];
