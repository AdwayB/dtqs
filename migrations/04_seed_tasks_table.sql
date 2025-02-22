INSERT INTO tasks (task_type, payload, status, priority, progress, attempts)
VALUES
    ('email', '{"to": "user@example.com", "subject": "Welcome!"}', 'completed', 1, 100, 1),
    ('video', '{"input": "video.mp4", "resolution": "720p"}', 'pending', 5, 0, 0),
    ('image', '{"input": "image.png", "filter": "grayscale"}', 'pending', 5, 0, 0);