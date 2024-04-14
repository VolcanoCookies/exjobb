import { configDotenv } from "dotenv";
import { connectDb } from "./db.js";
import {
  TrafikverketFlowEntryModel,
  TrafikverketSiteEntryModel,
  trafikverketFlowEntrySchema,
} from "./model/trafikverketFlowModel.js";
import { Point } from "./index.js";

configDotenv();

async function main() {
  await connectDb();

  let processed = 0;

  const query = {
    ModifiedTime: { $lte: "2024-03-23T00:00:00.000Z" },
  };

  await TrafikverketFlowEntryModel.deleteMany(query).exec();
  console.log("Deleted old entries");
}

export default main();
