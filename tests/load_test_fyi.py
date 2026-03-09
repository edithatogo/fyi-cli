"""Load testing for FYI Request System webapp using Locust.

Install: pip install locust
Run: locust -f load_test_fyi.py --headless -u 100 -r 10 --run-time 60s --host http://localhost:8000

This simulates multiple users accessing the FYI system simultaneously.
"""
from locust import HttpUser, task, between, events
import random
import json


class FYIWebUser(HttpUser):
    """Simulates a user interacting with the FYI web interface."""
    
    wait_time = between(1, 3)  # Wait 1-3 seconds between tasks
    
    @task(3)
    def view_dashboard(self):
        """User views the dashboard."""
        self.client.get("/")
    
    @task(2)
    def view_requests(self):
        """User views the requests list."""
        self.client.get("/requests")
    
    @task(2)
    def view_authorities(self):
        """User views the authorities list."""
        self.client.get("/authorities")
    
    @task(1)
    def view_timeline(self):
        """User views a request timeline (simulated)."""
        request_id = random.randint(1, 100)
        self.client.get(f"/requests/{request_id}")
    
    @task(1)
    def search_requests(self):
        """User searches for requests."""
        search_terms = ['test', 'request', 'fyi', 'official']
        term = random.choice(search_terms)
        self.client.get(f"/requests?q={term}")


class FYIAPIUser(HttpUser):
    """Simulates API client accessing FYI endpoints."""
    
    wait_time = between(0.5, 2)  # API calls are faster
    
    @task(5)
    def get_requests_json(self):
        """API client fetches requests as JSON."""
        self.client.get(
            "/requests",
            headers={"Accept": "application/json"}
        )
    
    @task(3)
    def get_request_detail(self):
        """API client fetches specific request."""
        request_id = random.randint(1, 100)
        self.client.get(
            f"/requests/{request_id}",
            headers={"Accept": "application/json"}
        )
    
    @task(2)
    def get_attention_report(self):
        """API client fetches attention report."""
        self.client.get(
            "/api/attention-report",
            headers={"Accept": "application/json"}
        )


@events.test_start.add_listener
def on_test_start(environment, **kwargs):
    """Called when load test starts."""
    print("\n" + "=" * 60)
    print("FYI REQUEST SYSTEM - LOAD TEST STARTING")
    print("=" * 60)
    print(f"Target host: {environment.host}")
    print("=" * 60 + "\n")


@events.test_stop.add_listener
def on_test_stop(environment, **kwargs):
    """Called when load test stops."""
    print("\n" + "=" * 60)
    print("FYI REQUEST SYSTEM - LOAD TEST COMPLETE")
    print("=" * 60)
    
    # Print statistics
    stats = environment.stats
    print(f"\nTotal requests: {stats.total.num_requests}")
    print(f"Failed requests: {stats.total.num_failures}")
    print(f"Average response time: {stats.total.avg_response_time:.2f}ms")
    print(f"Requests/second: {stats.total.current_rps:.2f}")
    
    # Check for failures
    if stats.total.num_failures > 0:
        print(f"\n⚠ WARNING: {stats.total.fail_ratio*100:.1f}% of requests failed!")
        for failure in stats.errors:
            print(f"  - {failure}")
    else:
        print("\n✓ All requests succeeded!")
    
    print("=" * 60 + "\n")


if __name__ == "__main__":
    import os
    os.system("locust -f load_test_fyi.py --host http://localhost:8000")
