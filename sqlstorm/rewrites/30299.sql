
WITH RECURSIVE UserReputationCTE AS (
    SELECT 
        Id,
        Reputation,
        CreationDate,
        LastAccessDate,
        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank
    FROM Users
),
PostStats AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Score,
        p.ViewCount,
        p.CreationDate,
        p.LastActivityDate,
        u.DisplayName AS OwnerDisplayName,
        COUNT(c.Id) AS CommentCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
    FROM Posts p
    LEFT JOIN Users u ON p.OwnerUserId = u.Id
    LEFT JOIN Comments c ON p.Id = c.PostId
    LEFT JOIN Votes v ON p.Id = v.PostId
    WHERE p.PostTypeId = 1 
    GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, u.DisplayName
),
PostHistoryStats AS (
    SELECT 
        ph.PostId,
        COUNT(ph.Id) AS HistoryCount,
        MAX(ph.CreationDate) AS LastEditDate
    FROM PostHistory ph
    WHERE ph.PostHistoryTypeId IN (4, 5, 6) 
    GROUP BY ph.PostId
),
UserBadges AS (
    SELECT 
        UserId,
        COUNT(*) AS BadgeCount,
        MAX(Date) AS LastBadgeDate,
        STRING_AGG(Name, ', ' ORDER BY Id) AS BadgeNames
    FROM Badges
    WHERE Class = 1 
    GROUP BY UserId
)
SELECT 
    us.Id AS UserId,
    us.DisplayName,
    us.Reputation,
    COALESCE(u.BadgeCount, 0) AS GoldBadgeCount,
    COALESCE(u.BadgeNames, '') AS GoldBadges,
    ps.PostId,
    ps.Title,
    ps.Score,
    ps.ViewCount,
    ps.CommentCount,
    ps.TotalUpvotes,
    ps.TotalDownvotes,
    COALESCE(ph.HistoryCount, 0) AS EditHistoryCount,
    ph.LastEditDate,
    DENSE_RANK() OVER (ORDER BY us.Reputation DESC) AS UserRank
FROM Users us
LEFT JOIN UserBadges u ON us.Id = u.UserId
LEFT JOIN PostStats ps ON us.DisplayName = ps.OwnerDisplayName
LEFT JOIN PostHistoryStats ph ON ps.PostId = ph.PostId
WHERE us.CreationDate > '2020-01-01' 
AND (u.BadgeCount IS NULL OR u.BadgeCount > 0) 
ORDER BY UserRank, ps.ViewCount DESC
LIMIT 100;
