from datetime import timedelta


def format_elapsed_time(elapsed_time: float) -> str:
    td = timedelta(seconds=elapsed_time)
    hours, reminder = divmod(td.seconds, 3600)
    minutes, seconds = divmod(reminder, 60)
    return f"{td.days}d{hours:02}h{minutes:02}m{seconds:02}s"
