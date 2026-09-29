
WITH UserBadgeCounts AS (
    SELECT UserId, 
           COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
           COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
           COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
    FROM Badges
    GROUP BY UserId
),
PostDetails AS (
    SELECT p.Id AS PostId, 
           p.Title, 
           p.Score, 
           p.CreationDate,
           COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
           COUNT(c.Id) AS CommentCount,
           SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
           SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
    FROM Posts p
    LEFT JOIN Comments c ON p.Id = c.PostId
    LEFT JOIN Votes v ON p.Id = v.PostId
    WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
    GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.AcceptedAnswerId
),
RankedPosts AS (
    SELECT pd.*, 
           RANK() OVER (ORDER BY pd.Score DESC) AS Rank
    FROM PostDetails pd
)
SELECT ub.UserId, 
       u.DisplayName, 
       ub.GoldBadges, 
       ub.SilverBadges, 
       ub.BronzeBadges,
       rp.PostId, 
       rp.Title, 
       rp.Score, 
       rp.Rank, 
       rp.CommentCount, 
       rp.Upvotes, 
       rp.Downvotes
FROM UserBadgeCounts ub
JOIN Users u ON ub.UserId = u.Id
LEFT JOIN RankedPosts rp ON u.Id = rp.AcceptedAnswerId
WHERE ub.GoldBadges > 0
  AND NOT EXISTS (
        SELECT 1 
        FROM Posts p 
        WHERE p.OwnerUserId = u.Id 
          AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
    )
ORDER BY ub.GoldBadges DESC, rp.Score DESC, ub.UserId, rp.PostId
LIMIT 10;
