"""Build a SENTIL trace from a pandas DataFrame."""

from __future__ import annotations

from typing import Optional, Sequence

try:
    import pandas as pd
except ModuleNotFoundError as exc:
    raise ModuleNotFoundError(
        "sentil.pandas needs pandas; install it with `pip install sentil[pandas]`"
    ) from exc

from ._sentil import Trace

__all__ = ["trace_from_dataframe"]

def trace_from_dataframe(
    df: "pd.DataFrame",
    time_column: str = "time",
    value_columns: Optional[Sequence[str]] = None,
) -> Trace:
    """Read a trace from a DataFrame."""
    if time_column not in df.columns:
        raise KeyError(f"time column '{time_column}' is not in the dataframe")
    if value_columns is None:
        value_columns = [column for column in df.columns if column != time_column]
    else:
        missing = [column for column in value_columns if column not in df.columns]
        if missing:
            raise KeyError(f"column(s) not in the dataframe: {', '.join(missing)}")
    times = df[time_column].to_numpy(dtype=float)
    signals = {column: df[column].to_numpy(dtype=float) for column in value_columns}
    return Trace(times, signals)