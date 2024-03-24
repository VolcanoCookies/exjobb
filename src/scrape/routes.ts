import { configDotenv } from "dotenv";
import { sleep } from "../lib/utils.js";
import { connectDb } from "../db.js";
import { BingClient } from "../lib/bing/client.js";
import { TomTomClient } from "../lib/tomtom/client.js";
import { HereClient } from "../lib/here/client.js";
import { Point } from "../index.js";
import { get_bearing } from "../utils.js";
import { saveResponse as saveRouteResponse } from "../model/routeModel.js";
import { Logger } from "@pieropatron/tinylogger";
import { AxiosError } from "axios";

configDotenv();
const logger = new Logger("scrape-routes");

async function main() {
  await connectDb();

  const bingKey = process.env.BING_API_KEY;
  const tomtomKey = process.env.TOMTOM_API_KEY;
  const hereKey = process.env.HERE_API_KEY;

  if (!bingKey) {
    throw new Error("BING_API_KEY not found");
  }
  if (!tomtomKey) {
    throw new Error("TOMTOM_API_KEY not found");
  }
  if (!hereKey) {
    throw new Error("HERE_API_KEY not found");
  }

  const bingClient = new BingClient(bingKey);
  const tomtomClient = new TomTomClient(tomtomKey);
  const hereClient = new HereClient(hereKey);

  const routes: { points: Point[]; heading: number }[] = [
    [
      { latitude: 59.2963961206535, longitude: 18.06193878000687 },
      { latitude: 59.296385164428706, longitude: 18.051081197914584 },
    ],
    [
      { latitude: 59.32399956066942, longitude: 17.895681500869905 },
      { latitude: 59.32412682317982, longitude: 17.896459341484224 },
    ],
    [
      { latitude: 59.32704073819411, longitude: 18.06248873525549 },
      { latitude: 59.328573193149495, longitude: 18.060005009707698 },
      { latitude: 59.32981070700758, longitude: 18.054337517696762 },
      { latitude: 59.33468561402644, longitude: 18.049494867235886 },
    ],
  ].map((points) => ({
    points,
    heading: get_bearing(points[0], points[1]),
  }));

  let bingFrequency = 1 * 60 * 1000;
  let tomtomFrequency = 5 * 60 * 1000;
  let hereFrequency = 5 * 60 * 1000;

  let duration = 24 * 60 * 60 * 1000;

  exitDelay(duration);

  const bingLogger = new Logger("bing");
  const tomtomLogger = new Logger("tomtom");
  const hereLogger = new Logger("here");

  setInterval(async () => {
    for (const route of routes) {
      try {
        const res = await bingClient.getRoute(route.points, route.heading);
        await saveRouteResponse(undefined, route.points, res);
      } catch (error) {
        if (error instanceof AxiosError) {
          bingLogger.warn(
            `HTTP error while scraping bing, status: ${error.response?.status}, message: ${error.response?.data}`
          );
        } else {
          bingLogger.warn("Request error while scraping bing");
          bingLogger.error(error);
        }
      }
    }
  }, bingFrequency);

  setInterval(async () => {
    for (const route of routes) {
      try {
        const res = await tomtomClient.getRoute(route.points, route.heading);
        await saveRouteResponse(undefined, route.points, res);
      } catch (error) {
        if (error instanceof AxiosError) {
          tomtomLogger.warn(
            `HTTP error while scraping tomtom, status: ${error.response?.status}, message: ${error.response?.data}`
          );
        } else {
          tomtomLogger.warn("Request error while scraping tomtom");
          tomtomLogger.error(error);
        }
      }
    }
  }, tomtomFrequency);
  setInterval(async () => {
    for (const route of routes) {
      try {
        const res = await hereClient.getRoute(
          route.points[0],
          route.points[1],
          undefined,
          route.heading
        );
        await saveRouteResponse(undefined, route.points, res);
      } catch (error) {
        if (error instanceof AxiosError) {
          hereLogger.warn(
            `HTTP error while scraping here, status: ${error.response?.status}, message: ${error.response?.data}`
          );
        } else {
          hereLogger.warn("Request error while scraping here");
          hereLogger.error(error);
        }
      }
    }
  }, hereFrequency);
}

async function exitDelay(delay: number) {
  await sleep(delay);
  process.exit(0);
}

export default main();
