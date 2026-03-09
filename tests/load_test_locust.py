"""Load testing for FYI CLI using Locust.

Simulates concurrent users performing typical operations.

Usage:
    locust -f tests/load_test_fyi.py --headless -u 100 -r 10 --run-time 60s --host http://localhost:8000
"""

from locust import HttpUser, task, between, events
import json
import random
import string


class FYIUser(HttpUser):
    """Simulates a FYI CLI user."""
    
    wait_time = between(1, 3)
    
    def on_start(self):
        """Initialize user session."""
        # Simulate login/setup
        self.request_id = None
        
    @task(3)
    def view_dashboard(self):
        """View dashboard."""
        self.client.get("/")
        
    @task(5)
    def list_requests(self):
        """List all requests."""
        self.client.get("/requests")
        
    @task(4)
    def view_request_detail(self):
        """View a specific request."""
        request_id = random.randint(1, 100)
        self.client.get(f"/requests/{request_id}")
        
    @task(2)
    def search_requests(self):
        """Search requests."""
        terms = ["ministry", "official", "information", "request"]
        query = random.choice(terms)
        self.client.get(f"/requests?q={query}")
        
    @task(1)
    def view_authorities(self):
        """View authorities list."""
        self.client.get("/authorities")


class APIUser(HttpUser):
    """Simulates API usage."""
    
    wait_time = between(0.5, 2)
    
    @task(5)
    def get_dashboard_json(self):
        """Get dashboard JSON."""
        self.client.get("/api/dashboard")
        
    @task(3)
    def get_requests_json(self):
        """Get requests JSON."""
        self.client.get("/api/requests")
        
    @task(2)
    def get_request_detail_json(self):
        """Get request detail JSON."""
        request_id = random.randint(1, 100)
        self.client.get(f"/api/request/{request_id}")


@events.test_start.add_listener
def on_test_start(environment, **kwargs):
    """Setup test environment."""
    print("Load test starting...")
    

@events.test_stop.add_listener
def on_test_stop(environment, **kwargs):
    """Cleanup after test."""
    print("Load test completed.")
    print(f"Total requests: {environment.stats.total.num_requests}")
    print(f"Failed requests: {environment.stats.total.num_failures}")
    

@events.request.add_listener
def on_request(request_type, name, response_time, response_length, response, 
               context, exception, start_time, url, **kwargs):
    """Log each request."""
    if exception:
        print(f"Request failed: {name} - {exception}")
