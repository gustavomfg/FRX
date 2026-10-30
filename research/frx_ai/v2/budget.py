"""Aggregate only timed experiment records, excluding annotation queues."""
import json


def spent(root):
    total=0.
    for folder in ['runs','baselines','active','retriever-runs']:
        for path in (root/folder).glob('*.json'):
            record=json.loads(path.read_text())
            if isinstance(record,dict) and isinstance(record.get('seconds'),(int,float)):
                total+=max(0.,record['seconds'])
    return total
