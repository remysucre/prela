
WITH UserInteractions AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(DISTINCT p.Id) AS PostCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
        MAX(p.CreationDate) AS LastPostDate,
        u.Reputation
    FROM Users u
    LEFT JOIN Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN Votes v ON p.Id = v.PostId
    GROUP BY u.Id, u.DisplayName, u.Reputation
),
PostDetails AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.LastActivityDate,
        COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
        pt.Name AS PostType,
        COUNT(c.Id) AS CommentCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
    FROM Posts p
    LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
    LEFT JOIN Comments c ON p.Id = c.PostId
    LEFT JOIN Votes v ON p.Id = v.PostId
    GROUP BY p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.AcceptedAnswerId, pt.Name
),
TopPosts AS (
    SELECT 
        pd.PostId,
        pd.Title,
        pd.CreationDate,
        pd.LastActivityDate,
        pd.AcceptedAnswerId,
        pd.PostType,
        pd.CommentCount,
        pd.Upvotes,
        pd.Downvotes,
        ROW_NUMBER() OVER (PARTITION BY pd.PostType ORDER BY pd.Upvotes DESC, pd.PostId) AS Ranking
    FROM PostDetails pd
)
SELECT 
    ui.DisplayName,
    tp.Title,
    tp.PostType,
    tp.CommentCount,
    tp.Upvotes,
    tp.Downvotes,
    tp.LastActivityDate,
    (SELECT COUNT(*) FROM Users u WHERE u.Reputation > ui.Reputation) AS HigherReputationCount
FROM UserInteractions ui
JOIN TopPosts tp ON ui.PostCount > 5 AND tp.Ranking <= 10
WHERE tp.LastActivityDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
ORDER BY ui.Reputation DESC, tp.Upvotes DESC;
