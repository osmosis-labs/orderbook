# Overview

This repo contains the core contract for Osmosis's orderbook. It implements a novel orderbook mechanism that achieves constant time complexity on limit placements while maintaining log-time complexity for cancellations. Swaps are constant time for each tick, protecting the mechanism against many common spam vectors that naive queue-based orderbooks are vulnerable to.