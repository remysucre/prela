
WITH UserBadges AS (
    SELECT 
        b.UserId,
        COUNT(*) AS TotalBadges,
        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
    FROM 
        Badges b
    GROUP BY 
        b.UserId
),
FilteredPosts AS (
    SELECT 
        p.Id AS PostId,
        p.OwnerUserId,
        p.CreationDate,
        p.Score,
        p.AnswerCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS UserPostRank
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
        AND p.Score > 0
),
PostVoteSummary AS (
    SELECT 
        v.PostId,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        COUNT(v.UserId) AS TotalVotes
    FROM 
        Votes v
    GROUP BY 
        v.PostId
),
ClosePostReasons AS (
    SELECT 
        ph.PostId,
        STRING_AGG(cr.Name, ', ') AS CloseReasons
    FROM 
        PostHistory ph
    JOIN 
        CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
    WHERE 
        ph.PostHistoryTypeId = 10 
    GROUP BY 
        ph.PostId
)
SELECT 
    u.DisplayName,
    u.Reputation,
    ub.TotalBadges,
    ub.GoldBadges,
    ub.SilverBadges,
    ub.BronzeBadges,
    fp.PostId,
    fp.CreationDate AS PostCreationDate,
    fp.Score AS PostScore,
    pvs.UpVotes,
    pvs.DownVotes,
    pvs.TotalVotes,
    COALESCE(cpr.CloseReasons, 'No close reasons') AS CloseReasonDescriptions,
    CASE 
        WHEN ub.TotalBadges IS NULL THEN 'No badges yet!'
        WHEN ub.TotalBadges > 10 THEN 'Super user'
        ELSE 'Regular user'
    END AS UserType,
    LEAD(fp.Score) OVER (PARTITION BY fp.OwnerUserId ORDER BY fp.CreationDate) AS NextPostScore
FROM 
    Users u
LEFT JOIN 
    UserBadges ub ON u.Id = ub.UserId
LEFT JOIN 
    FilteredPosts fp ON u.Id = fp.OwnerUserId
LEFT JOIN 
    PostVoteSummary pvs ON fp.PostId = pvs.PostId
LEFT JOIN 
    ClosePostReasons cpr ON fp.PostId = cpr.PostId
WHERE 
    u.Reputation >= 1000
    AND (fp.UserPostRank = 1 OR fp.UserPostRank IS NULL)
ORDER BY 
    ub.TotalBadges DESC,
    fp.CreationDate DESC
LIMIT 100 OFFSET 0;
