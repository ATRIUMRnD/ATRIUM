import os, time, sys
os.environ["LIMEN_LOCAL_GPU"] = "1"
sys.path.insert(0, "/mnt/d/LIMEN")
from limen.router import RouteRequest, route_local_gpu_first, query_free_vram_mib, gpu_qubit_cap
from limen.pipeline import run_route_request
free = query_free_vram_mib(); print("free MiB", free, "cap", gpu_qubit_cap(free))
n = int(sys.argv[1])
qubo = {(f"x{i}", f"x{i+1}"): 1.0 for i in range(n-1)}
qubo.update({(f"x{i}", f"x{i}"): -1.0 for i in range(n)})
req = RouteRequest(qubo, fidelity_target=0.9, credit_budget=0.0)
plan = route_local_gpu_first(req)
print("plan:", plan.backend.name, plan.tier, plan.pipeline_kwargs); [print("  note:", x) for x in plan.notes]
t = time.time()
cert = run_route_request(req, emit_report=True, memory=None)
print("elapsed %.1fs" % (time.time()-t))
print("backend:", cert.metadata.get("execution_backend"), "energy", cert.energy if hasattr(cert,"energy") else None, "optimal", cert.is_optimal)
print("report backend:", cert.metadata["route_report"]["backend"], "tier", cert.metadata["route_report"]["tier"])
