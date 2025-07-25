import type { NextRequest } from 'next/server';
import { NextResponse } from 'next/server';
import { submitQuery } from '../../actions/query';

export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const { query } = body;

    if (!query || typeof query !== 'string') {
      return NextResponse.json(
        { error: 'Query is required and must be a string' },
        { status: 400 },
      );
    }

    const result = await submitQuery(query);
    return NextResponse.json(result);
  } catch (error) {
    console.error('Error in doc-chat API:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 },
    );
  }
}
