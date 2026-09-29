
WITH UserBadgeCounts AS (
    SELECT 
        u.Id AS UserId,
        COUNT(b.Id) AS BadgeCount,
        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
    FROM Users u
    LEFT JOIN Badges b ON u.Id = b.UserId
    GROUP BY u.Id
),
PostStatistics AS (
    SELECT 
        p.OwnerUserId,
        COUNT(p.Id) AS PostCount,
        COALESCE(SUM(p.Score), 0) AS TotalScore,
        COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
        MAX(p.CreationDate) AS MostRecentPostDate
    FROM Posts p
    GROUP BY p.OwnerUserId
),
CombinedStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COALESCE(ubc.BadgeCount, 0) AS BadgeCount,
        COALESCE(ps.PostCount, 0) AS PostCount,
        COALESCE(ps.TotalScore, 0) AS TotalScore,
        COALESCE(ps.TotalViews, 0) AS TotalViews,
        ps.MostRecentPostDate
    FROM Users u
    LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId
    LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId
)
SELECT 
    *,
    CASE 
        WHEN BadgeCount > 0 AND PostCount > 0 THEN 'Active'
        WHEN BadgeCount > 0 AND PostCount = 0 THEN 'Badge Holder, No Posts'
        WHEN BadgeCount = 0 AND PostCount > 0 THEN 'Inactive Badge Seeker'
        ELSE 'New User'
    END AS UserStatus,
    ROW_NUMBER() OVER (PARTITION BY 
        CASE 
            WHEN BadgeCount > 0 AND PostCount > 0 THEN 'Active'
            WHEN BadgeCount > 0 AND PostCount = 0 THEN 'Badge Holder, No Posts'
            WHEN BadgeCount = 0 AND PostCount > 0 THEN 'Inactive Badge Seeker'
            ELSE 'New User' 
        END 
        ORDER BY TotalScore DESC, UserId) AS StatusRank
FROM CombinedStats
WHERE UserId IS NOT NULL
ORDER BY UserStatus, StatusRank
LIMIT 100;
