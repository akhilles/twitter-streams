# Twitter Streams

## Deploy

```bash
# build image
docker build -t twitter-streams .
# run image w/ credentials injected via environment variables
docker run -d --rm -p 8080:8080 -e TWITTER_API_KEY -e TWITTER_API_SECRET_KEY -e TWITTER_API_ACCESS_TOKEN -e TWITTER_API_ACCESS_TOKEN_SECRET twitter-streams
```

## Architecture/Design

### Processing Twitter Stream

There are 2 primary challenges in processing the high-volume Twitter stream:
1. Compute-heavy functionality (such as JSON decoding) should be offloaded from the task that is reading bytes of the wire.
2. Tweets need to be processed in parallel. Raw tweet payloads in the form of `Stream<Chunk>` need to be processed in order because some tweets can span across multiple HTTP body chunks.

These challenges were addressed by pre-processing `Stream<Chunk>` into `Stream<Tweet>` (which can be processed in parallel), and then draining it into a buffered `mspc::channel`. On the receiving end of the `channel`, a worker pool can process the `Stream<Tweet>` and store the processed tweet. Retweets are filtered out along with tweets where the keyword is in the tweet metadata but not the tweet body itself. Only the 400 most recent processed tweets are stored.

### Serving the Charts

A simple http server (running on the same `tokio` runtime as the stream processor) is used to serve a single static html file along with 2 endpoints: `/back_pressure` and `/keyword_frequency`. Some client-side Javascript fetches chart data form these endpoints every ~2 seconds and renders 3 charts onto HTML5 canvases using https://www.chartjs.org.
