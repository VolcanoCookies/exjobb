import mongoose from 'mongoose';

export async function connectDb() {
	const mongoUrl = process.env.MONGO_URL;
	if (!mongoUrl) {
		throw new Error('MONGO_URL not found');
	}

	await mongoose.connect(mongoUrl, {
		dbName: 'exjobb',
	});

	console.log('Connected to MongoDB');
}
