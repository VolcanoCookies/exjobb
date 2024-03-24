import { configDotenv } from "dotenv";
import { connectDb } from "./db.js";
import {
  TrafikverketFlowEntryModel,
  TrafikverketSiteEntryModel,
} from "./model/trafikverketFlowModel.js";
import { Point } from "./index.js";

configDotenv();

async function main() {
  await connectDb();

  let total = await TrafikverketFlowEntryModel.countDocuments().exec();
  let processed = 0;

  const sites = new Map<number, Point>();

  const cursor = TrafikverketFlowEntryModel.find().cursor();
  for (let doc = await cursor.next(); doc != null; doc = await cursor.next()) {
    processed++;
    if (processed % 10000 === 0)
      console.log(`Processed ${processed} of ${total}`);

    if (!sites.has(doc.SiteId)) {
      sites.set(doc.SiteId, {
        latitude: doc.location.coordinates[1],
        longitude: doc.location.coordinates[0],
      });
    } else {
      const site = sites.get(doc.SiteId)!;
      if (
        site.latitude !== doc.location.coordinates[1] ||
        site.longitude !== doc.location.coordinates[0]
      ) {
        console.error(`Site ${doc.SiteId} has different coordinates`);
        process.exit(1);
      }
    }
  }

  console.log(`Creating ${sites.size} site entries`);

  sites.forEach((point, siteId) => {
    TrafikverketSiteEntryModel.create({
      SiteId: siteId,
      location: {
        type: "Point",
        coordinates: [point.longitude, point.latitude],
      },
    });
  });
}

export default main();
