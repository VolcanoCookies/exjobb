import { Logger } from "@pieropatron/tinylogger";
import mongoose from "mongoose";

const logger = new Logger("db");

export async function connectDb() {
  const mongoUrl = process.env.MONGO_URL;
  if (!mongoUrl) {
    logger.error("MONGO_URL not found");
    process.exit(1);
  }

  await mongoose.connect(mongoUrl, {
    dbName: "exjobb_demo",
  });

  logger.info("Connected to MongoDB");
}
